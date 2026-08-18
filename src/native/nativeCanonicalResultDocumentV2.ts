import {
  CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
  capabilityCellReferenceIdentityV2,
  canonicalResultDocumentFromLegacyTables,
  type CanonicalChartDisplayOptions,
  type CanonicalColumnType,
  type CanonicalResultCell,
  type CanonicalResultDocumentV2,
  type CanonicalResultNotice,
  type CanonicalResultProvenanceV2,
  type CanonicalResultSection,
  type CanonicalResultTable,
  type CapabilityCellReferenceV2,
  validateCanonicalResultDocumentV2,
} from "../domain/canonicalResultDocumentV2";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import {
  methodCapabilityRequirementsV2,
  type MethodCapabilityRequirementV2,
} from "../domain/methodCapabilityRegistryV2";
import {
  ESTABLISHED_METHOD_CONTRACTS_V1,
  establishedCanonicalTableOwnerOptionsV1,
} from "../domain/generated/establishedMethodContractsV1";
import {
  inspectNativeConstructAuthoringV4,
  inspectNativeCovarianceAuthoringV4,
} from "../domain/semModelV4Authoring";
import type { ResultTable } from "../domain/resultTables";
import type { AnalysisRun, AnalysisUiSettings } from "../types";
import {
  buildNativeResultNavigation,
  isCompletedResultRun,
  nativePlsPosthocMinimumSampleSizeProjection,
} from "./nativeResults";

export type NativeCanonicalResultModeV2 = "current_typed_bridge" | "historical_text_fallback";

export interface NativeCanonicalResultPresentationV2 {
  precision?: number;
  missingValueLabel?: string;
  chartDefaults?: CanonicalChartDisplayOptions;
}

export interface NativeCanonicalResultContextV2 {
  /** Project identity supplied by the runtime shell when one is available. */
  projectId?: string;
  /** Dataset identity supplied by the runtime shell when one is available. */
  datasetId?: string;
  presentation?: NativeCanonicalResultPresentationV2;
}

export const CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION = 1 as const;

export interface CanonicalGeneralSemResultTraceV1 {
  model_id: string;
  capability_cell: CapabilityCellReferenceV2;
}

export interface CanonicalGeneralSemEstimateV1 {
  estimate: number;
  standard_error?: number | null;
  lower?: number | null;
  upper?: number | null;
  p_value?: number | null;
}

export interface CanonicalSpecificIndirectEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  ordered_relation_ids: string[];
  value: CanonicalGeneralSemEstimateV1;
}

export type CanonicalAggregateEffectKindV1 = "total_indirect" | "total_effect";

export interface CanonicalAggregateEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  kind: CanonicalAggregateEffectKindV1;
  source_id: string;
  target_id: string;
  value: CanonicalGeneralSemEstimateV1;
}

export type CanonicalConditionalProbeValuesResultV1 =
  | { kind: "data_derived_mean_plus_minus_one_sd"; mean: number; standard_deviation: number }
  | { kind: "explicit"; values: number[] };

export interface CanonicalConditionalEffectProbeResultV1 {
  probe_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  moderator_id: string;
  values: CanonicalConditionalProbeValuesResultV1;
}

export interface CanonicalConditionalEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  probe_id: string;
  moderator_id: string;
  probe_value_index: number;
  moderator_value: number;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalInteractionPlotPointV1 {
  focal_value: number;
  predicted_value: number;
  lower?: number | null;
  upper?: number | null;
}

export interface CanonicalInteractionPlotSeriesV1 {
  series_id: string;
  probe_id: string;
  probe_value_index: number;
  moderator_value: number;
  points: CanonicalInteractionPlotPointV1[];
}

export interface CanonicalInteractionPlotResultV1 {
  plot_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  focal_predictor_id: string;
  moderator_id: string;
  outcome_id: string;
  series: CanonicalInteractionPlotSeriesV1[];
}

export type CanonicalHocStageKindV1 =
  | "lower_order_score_estimation"
  | "higher_order_estimation";

export interface CanonicalHocRelationEstimateV1 {
  relation_id: string;
  source_id: string;
  target_id: string;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalHocStageResultV1 {
  stage_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  higher_order_construct_id: string;
  stage_number: number;
  kind: CanonicalHocStageKindV1;
  input_construct_ids: string[];
  output_variable_ids: string[];
  relation_estimates?: CanonicalHocRelationEstimateV1[];
}

export interface CanonicalGeneralSemIntervalV1 {
  confidence_level: number;
  lower: number;
  upper: number;
}

export interface CanonicalCbsemFitResultV1 {
  fit_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  chi_square: number;
  degrees_of_freedom: number;
  chi_square_p_value?: number | null;
  rmsea?: number | null;
  rmsea_interval?: CanonicalGeneralSemIntervalV1 | null;
  cfi?: number | null;
  tli?: number | null;
  srmr?: number | null;
  aic?: number | null;
  bic?: number | null;
}

export type CanonicalIdentificationScopeV1 =
  | "model"
  | "variable"
  | "relation"
  | "interaction"
  | "higher_order_construct";

export type CanonicalIdentificationStatusV1 =
  | "identified"
  | "underidentified"
  | "locally_underidentified"
  | "boundary_condition";

export interface CanonicalIdentificationDiagnosticV1 {
  diagnostic_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  scope: CanonicalIdentificationScopeV1;
  subject_id: string;
  status: CanonicalIdentificationStatusV1;
  code: string;
  message: string;
  degrees_of_freedom?: number | null;
}

/** Empty collections are omitted by Rust, so every result family is optional on the wire. */
export interface CanonicalGeneralSemResultsV1 {
  schema_version: typeof CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION;
  specific_indirect_effects?: CanonicalSpecificIndirectEffectResultV1[];
  aggregate_effects?: CanonicalAggregateEffectResultV1[];
  conditional_effect_probes?: CanonicalConditionalEffectProbeResultV1[];
  conditional_effects?: CanonicalConditionalEffectResultV1[];
  interaction_plots?: CanonicalInteractionPlotResultV1[];
  higher_order_stages?: CanonicalHocStageResultV1[];
  cbsem_fit?: CanonicalCbsemFitResultV1[];
  identification_diagnostics?: CanonicalIdentificationDiagnosticV1[];
}

/** Additive cross-runtime wire shape; legacy documents omit general_sem_results entirely. */
export interface NativeCanonicalResultDocumentV2 extends CanonicalResultDocumentV2 {
  general_sem_results?: CanonicalGeneralSemResultsV1;
}

export type NativeCanonicalResultDocumentV2ParseErrorCode =
  | "schema.invalid_shape"
  | "schema.unknown_field"
  | "schema.invalid_discriminator"
  | "schema.version_unsupported"
  | "schema.non_finite"
  | "schema.integer_invalid"
  | "document.invalid";

export class NativeCanonicalResultDocumentV2ParseError extends Error {
  constructor(
    public readonly code: NativeCanonicalResultDocumentV2ParseErrorCode,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "NativeCanonicalResultDocumentV2ParseError";
  }
}

export type NativeCanonicalResultBuildV2 =
  | {
      ok: true;
      mode: NativeCanonicalResultModeV2;
      document: NativeCanonicalResultDocumentV2;
    }
  | {
      ok: false;
      code:
        | "run_not_completed"
        | "invalid_analytical_payload"
        | "invalid_native_result_tables"
        | "unresolved_capability_cell"
        | "invalid_canonical_document"
        | "technical_provenance_unavailable";
      errors: string[];
    };

const DEFAULT_PRECISION = 4;
const DEFAULT_MISSING_VALUE_LABEL = "—";
const STRICT_NUMBER = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i;
const STRICT_BOOLEAN = /^(?:true|false)$/i;
const RECORDED_DIGEST = /^(?:(?:sha256|v2):)?([a-f0-9]{64})$/;
type CanonicalMissingReason = "not_applicable" | "not_estimated" | "undefined" | "withheld";
const MISSING_VALUES = new Map<string, CanonicalMissingReason>([
  ["", "not_estimated"],
  ["—", "not_estimated"],
  ["â€”", "not_estimated"],
  ["na", "not_applicable"],
  ["n/a", "not_applicable"],
  ["not applicable", "not_applicable"],
  ["not estimated", "not_estimated"],
  ["undefined", "undefined"],
  ["withheld", "withheld"],
]);

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

async function sha256Hex(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function recordedOrDerivedDigest(value: string, namespace: string): Promise<string> {
  const recorded = RECORDED_DIGEST.exec(value.trim().toLowerCase());
  return recorded?.[1] ?? sha256Hex(`${namespace}\u0000${value}`);
}

interface ScientificModelEdgeProjection {
  id: string;
  source: string;
  target: string;
  source_handle: string | null;
  target_handle: string | null;
  role: unknown;
  control_label?: unknown;
  covariance?: unknown;
}

function scientificModelProjection(run: AnalysisRun): unknown {
  const snapshot = run.modelSnapshot;
  if (!snapshot) return { model_id: run.modelId ?? null, model_snapshot: "not_recorded" };
  return {
    nodes: [...snapshot.nodes]
      .map((node) => ({
        id: node.id,
        type: node.type ?? null,
        mode: node.data.mode,
        indicators: [...node.data.indicators],
        semantic: node.data.semantic ?? null,
        interaction: node.data.interaction ?? null,
        higher_order: node.data.higherOrder ?? null,
        estimand: (() => {
          const inspection = inspectNativeConstructAuthoringV4(node);
          return inspection.state === "invalid"
            ? { kind: "invalid_authoring_metadata" }
            : inspection.specification;
        })(),
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
    edges: [...snapshot.edges]
      .flatMap<ScientificModelEdgeProjection>((edge) => {
        const role = (edge.data as { role?: unknown } | undefined)?.role ?? null;
        if (role === "covariance") {
          const inspection = inspectNativeCovarianceAuthoringV4(edge);
          if (inspection.state === "presentation_only" || inspection.state === "legacy_unspecified") return [];
          return [{
            id: edge.id,
            source: edge.source,
            target: edge.target,
            source_handle: edge.sourceHandle ?? null,
            target_handle: edge.targetHandle ?? null,
            role,
            covariance: inspection.state === "invalid"
              ? { kind: "invalid_authoring_metadata" }
              : {
                  kind: "scientific",
                  left: inspection.specification.left,
                  right: inspection.specification.right,
                },
          }];
        }
        return [{
          id: edge.id,
          source: edge.source,
          target: edge.target,
          source_handle: edge.sourceHandle ?? null,
          target_handle: edge.targetHandle ?? null,
          role,
          control_label: (edge.data as { controlLabel?: unknown } | undefined)?.controlLabel ?? null,
        }];
      })
      .sort((left, right) => left.id.localeCompare(right.id)),
  };
}

export function nativeAnalysisUiSettingsForRunV2(run: AnalysisRun): AnalysisUiSettings | null {
  const settings = run.provenance?.settings;
  if (!settings) return null;
  const regressionType = run.result?.regression?.regression_type;
  return {
    method: settings.method,
    weightingScheme: settings.weighting_scheme,
    tolerance: settings.tolerance,
    maxIterations: settings.max_iterations,
    preprocessing: settings.preprocessing,
    bootstrapSamples: settings.bootstrap_samples,
    studentizedInnerSamples: settings.studentized_inner_samples,
    permutationSamples: settings.permutation_samples,
    seed: settings.seed,
    workers: settings.workers,
    confidenceLevel: settings.confidence_level,
    caseWeightColumn: settings.case_weight_column,
    cbsemModelType: run.result?.cbsem?.model_type.toLowerCase() === "cfa" ? "cfa" : "sem",
    cbsemGroupColumn: run.result?.cbsem?.multigroup?.group_column ?? null,
    cbsemInvarianceSteps: run.result?.cbsem?.multigroup?.invariance.map((row) => row.step).join(",") || null,
    cbsemBootstrapSamples: run.result?.cbsem?.bootstrap_v2 ? settings.bootstrap_samples : 0,
    regressionType: regressionType === "logistic" || regressionType === "ols" ? regressionType : "process",
    regressionBootstrap: Boolean(run.result?.regression?.bootstrap),
  };
}

function historicalRequirement(run: AnalysisRun): MethodCapabilityRequirementV2 {
  if (run.result?.gsca) return { capability_id: "smartpls.gsca", cell_id: "qpls3.gsca.als", option: "gsca" };
  if (run.result?.cbsem?.bootstrap_v2 || run.result?.cbsem?.bootstrap) {
    return { capability_id: "smartpls.cbsem_bootstrapping", cell_id: "qpls3.cbsem.bootstrap", option: "cbsem_bootstrap" };
  }
  if (run.result?.cbsem?.model_type.toLowerCase() === "cfa") {
    return { capability_id: "smartpls.cfa", cell_id: "qpls3.cbsem.ml", option: "cfa_ml" };
  }
  if (run.result?.cbsem) return { capability_id: "smartpls.cbsem", cell_id: "qpls3.cbsem.ml", option: "sem_ml" };
  if (run.result?.pca) return { capability_id: "smartpls.pca_core", cell_id: "qpls3.standalone.pca", option: "pca" };
  if (run.result?.nca) return { capability_id: "smartpls.nca", cell_id: "qpls3.standalone.nca", option: "nca" };
  if (run.result?.cta_pls) return { capability_id: "smartpls.cta_pls", cell_id: "qpls3.assessment.cta_pls", option: "cta_pls" };
  if (run.result?.cca) return { capability_id: "smartpls.cca", cell_id: "qpls3.assessment.cca_residuals", option: "cca" };
  if (run.result?.endogeneity) {
    return { capability_id: "smartpls.endogeneity_gaussian_copulas", cell_id: "qpls3.pls.gaussian_copula_endogeneity", option: "gaussian_copula_endogeneity" };
  }
  if (run.result?.plsc) return { capability_id: "smartpls.plsc", cell_id: "qpls3.pls.consistent", option: "consistent_pls" };
  if (run.result?.wpls) return { capability_id: "smartpls.wpls", cell_id: "qpls3.pls.weighted", option: "weighted_pls" };
  if (run.bootstrap) return { capability_id: "smartpls.pls_bootstrapping", cell_id: "qpls3.inference.bootstrap", option: "pls_bootstrap" };
  if (run.permutation) {
    return { capability_id: "smartpls.permutation", cell_id: "qpls3.inference.structural_path_randomization", option: "structural_path_randomization" };
  }
  return { capability_id: "smartpls.pls_algorithm", cell_id: "qpls3.pls.algorithm", option: "pls_algorithm" };
}

function resolveCapabilityCell(run: AnalysisRun): CapabilityCellReferenceV2 | null {
  const requirements = nativeCapabilityRequirementsForRunV2(run);
  const requirement = requirements.at(-1);
  return requirement ? capabilityCellForRequirement(requirement) : null;
}

export function nativeCapabilityRequirementsForRunV2(run: AnalysisRun): readonly MethodCapabilityRequirementV2[] {
  const settings = nativeAnalysisUiSettingsForRunV2(run);
  if (!settings) {
    return [historicalRequirement(run)];
  }
  try {
    return methodCapabilityRequirementsV2(settings);
  } catch {
    return [];
  }
}

function capabilityCellForRequirement(requirement: MethodCapabilityRequirementV2): CapabilityCellReferenceV2 | null {
  const matches = capabilityRegistryV2.quickPlsCell(requirement.cell_id).filter((match) => (
    match.row.capability_id === requirement.capability_id
    && match.cell.capability_id === requirement.capability_id
    && match.cell.cell_id === requirement.cell_id
    && match.link.capability_id === requirement.capability_id
  ));
  if (matches.length !== 1) return null;
  return { ...matches[0].link };
}

function requirement(capability_id: string, cell_id: string, option: string): MethodCapabilityRequirementV2 {
  return { capability_id, cell_id, option };
}

function establishedCanonicalTableRequirementsV1(
  tableId: string,
): readonly MethodCapabilityRequirementV2[] | null {
  const ownerOptions = establishedCanonicalTableOwnerOptionsV1(tableId);
  if (ownerOptions.length === 0) return null;
  return ownerOptions.flatMap((ownerOption) => ESTABLISHED_METHOD_CONTRACTS_V1.flatMap((contract) => (
    contract.capability_requirements
      .filter((item) => item.option === ownerOption)
      .map((item) => requirement(item.capability_id, item.cell_id, item.option))
  )));
}

function sortedDistinctCapabilityCells(
  references: readonly CapabilityCellReferenceV2[],
): CapabilityCellReferenceV2[] {
  const byIdentity = new Map(references.map((reference) => [capabilityCellReferenceIdentityV2(reference), reference]));
  return [...byIdentity.entries()]
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([, reference]) => ({ ...reference }));
}

export function nativeCapabilityRequirementsForTableV2(
  tableId: string,
): readonly MethodCapabilityRequirementV2[] | null {
  if (tableId === "posthoc_minimum_sample_size") {
    return [requirement("smartpls.pls_power_analysis", "qpls3.pls.posthoc_technical_minimum_sample_size", "post_hoc_sample_size")];
  }
  if (tableId === "blindfolding") {
    return [requirement("smartpls.blindfolding", "qpls3.assessment.blindfolding_legacy", "historical_blindfolding")];
  }
  if (["htmt", "htmt_plus", "htmt_original"].includes(tableId)) {
    return [requirement("smartpls.htmt", "qpls3.assessment.htmt", "htmt")];
  }
  if (["model_fit", "model_fit_details", "model_fit_exact", "model_fit_exact_failures"].includes(tableId)) {
    return [requirement("smartpls.model_fit", "qpls3.assessment.model_fit", "pls_model_fit")];
  }
  if (["specific_indirect_effects", "total_indirect_effects"].includes(tableId)) {
    return [requirement("smartpls.mediation", "qpls3.pls.mediation", "mediation")];
  }
  if (tableId === "mediation_bootstrap") {
    return [
      requirement("smartpls.mediation", "qpls3.pls.mediation", "mediation"),
      requirement("smartpls.pls_bootstrapping", "qpls3.inference.bootstrap", "pls_bootstrap"),
    ];
  }
  if (/^(?:bootstrap_|control_(?:bootstrap|bca|studentized))/.test(tableId)) {
    return [requirement("smartpls.pls_bootstrapping", "qpls3.inference.bootstrap", "pls_bootstrap")];
  }
  if (tableId === "permutation" || tableId === "control_randomization") {
    return [requirement("smartpls.permutation", "qpls3.inference.structural_path_randomization", "structural_path_randomization")];
  }
  if (tableId.startsWith("moderation_")) {
    const values = [requirement("smartpls.moderation", "qpls3.pls.moderation", "moderation")];
    if (tableId === "moderation_randomization") {
      values.push(requirement("smartpls.permutation", "qpls3.inference.structural_path_randomization", "structural_path_randomization"));
    } else if (["moderation_bootstrap", "moderation_bca", "moderation_studentized"].includes(tableId)) {
      values.push(requirement("smartpls.pls_bootstrapping", "qpls3.inference.bootstrap", "pls_bootstrap"));
    }
    return values;
  }
  if (tableId.startsWith("hoc_")) {
    return [requirement("smartpls.higher_order_models", "qpls3.pls.higher_order_two_stage", "higher_order")];
  }
  if (tableId.startsWith("plspredict_")) {
    return [requirement("smartpls.plspredict", "qpls3.prediction.plspredict_cvpat", "plspredict")];
  }
  if (tableId === "cvpat" || tableId.startsWith("cvpat_")) {
    return [requirement("smartpls.cvpat", "qpls3.prediction.plspredict_cvpat", "cvpat")];
  }
  if (tableId.startsWith("micom_")) {
    return [requirement("smartpls.micom", "qpls3.groups.micom_permutation_mga", "micom")];
  }
  if (tableId.startsWith("mga_")) {
    return [requirement("smartpls.mga", "qpls3.groups.micom_permutation_mga", "mga")];
  }
  const established = establishedCanonicalTableRequirementsV1(tableId);
  if (established) return established;
  if (tableId.startsWith("cta_pls_")) {
    return [requirement("smartpls.cta_pls", "qpls3.assessment.cta_pls", "cta_pls")];
  }
  if (tableId === "endogeneity_copula") {
    return [requirement("smartpls.endogeneity_gaussian_copulas", "qpls3.pls.gaussian_copula_endogeneity", "endogeneity")];
  }
  if (tableId.startsWith("plsc_permutation_")) {
    return [requirement(
      "smartpls.consistent_permutation",
      "qpls3.inference.consistent_permutation",
      "consistent_permutation",
    )];
  }
  if (tableId.startsWith("plsc_")) {
    return [requirement("smartpls.plsc", "qpls3.pls.consistent", "consistent_pls")];
  }
  if (tableId.startsWith("wpls_")) {
    return [requirement("smartpls.wpls", "qpls3.pls.weighted", "weighted_pls")];
  }
  if (tableId.startsWith("pca_")) {
    return [
      requirement("smartpls.pca_core", "qpls3.standalone.pca", "pca_core_catalogue_row"),
      requirement("smartpls.pca_cbsem", "qpls3.standalone.pca", "pca_cbsem_catalogue_row"),
    ];
  }
  if (tableId.startsWith("process_bootstrap_")) {
    return [requirement("smartpls.process_bootstrapping", "qpls3.standalone.process", "process_bootstrap")];
  }
  if (tableId.startsWith("process_") || tableId.startsWith("legacy_process_")) {
    return [requirement("smartpls.process", "qpls3.standalone.process", "process")];
  }
  if (tableId.startsWith("regression_bootstrap_")) {
    return [requirement("smartpls.regression_bootstrapping", "qpls3.standalone.regression_bootstrap", "regression_bootstrap")];
  }
  if (tableId.startsWith("logistic_") || tableId.startsWith("legacy_logistic_")) {
    return [requirement("smartpls.logistic_regression", "qpls3.standalone.logistic", "logistic")];
  }
  if (tableId.startsWith("ols_")) {
    return [requirement("smartpls.regression", "qpls3.standalone.ols", "ols")];
  }
  if (tableId.startsWith("cbsem_bootstrap_")) {
    return [requirement("smartpls.cbsem_bootstrapping", "qpls3.cbsem.bootstrap", "cbsem_bootstrap")];
  }
  return null;
}

function capabilityCellsForTable(
  run: AnalysisRun,
  tableId: string,
): CapabilityCellReferenceV2[] | null {
  const runRequirements = nativeCapabilityRequirementsForRunV2(run);
  const explicit = nativeCapabilityRequirementsForTableV2(tableId);
  const requirements = explicit ?? (runRequirements.length > 0 ? [runRequirements[0]] : []);
  const references = requirements.map(capabilityCellForRequirement);
  if (references.length === 0 || references.some((reference) => reference === null)) return null;
  return sortedDistinctCapabilityCells(references as CapabilityCellReferenceV2[]);
}

function hasNonFiniteNumber(value: unknown, seen = new Set<object>()): boolean {
  if (typeof value === "number") return !Number.isFinite(value);
  if (!value || typeof value !== "object") return false;
  if (seen.has(value)) return true;
  seen.add(value);
  const invalid = Object.values(value).some((child) => hasNonFiniteNumber(child, seen));
  seen.delete(value);
  return invalid;
}

type StrictWireRecord = Record<string, unknown>;

const GENERAL_SEM_STABLE_ID = /^[a-z0-9][a-z0-9_.:-]*$/;

function wireFail(
  code: NativeCanonicalResultDocumentV2ParseErrorCode,
  path: string,
  message: string,
): never {
  throw new NativeCanonicalResultDocumentV2ParseError(code, path, message);
}

function strictWireRecord(value: unknown, path: string): StrictWireRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return wireFail("schema.invalid_shape", path, `${path} must be an object.`);
  }
  return value as StrictWireRecord;
}

function exactWireRecord(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): StrictWireRecord {
  const record = strictWireRecord(value, path);
  const allowed = new Set([...required, ...optional]);
  const unknown = Object.keys(record).find((key) => !allowed.has(key));
  if (unknown) return wireFail("schema.unknown_field", `${path}.${unknown}`, `${path}.${unknown} is not supported.`);
  const missing = required.find((key) => !Object.prototype.hasOwnProperty.call(record, key));
  if (missing) return wireFail("schema.invalid_shape", `${path}.${missing}`, `${path}.${missing} is required.`);
  return record;
}

function wireArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) return wireFail("schema.invalid_shape", path, `${path} must be an array.`);
  return value;
}

function optionalWireArray(record: StrictWireRecord, key: string, path: string): unknown[] {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return [];
  return wireArray(record[key], `${path}.${key}`);
}

function wireText(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    return wireFail("schema.invalid_shape", path, `${path} must be nonempty text.`);
  }
  return value;
}

function wireStableId(value: unknown, path: string): string {
  const id = wireText(value, path);
  if (!GENERAL_SEM_STABLE_ID.test(id)) {
    return wireFail("document.invalid", path, `${path} must be a stable lowercase identifier.`);
  }
  return id;
}

function wireFinite(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return wireFail("schema.non_finite", path, `${path} must be a finite number.`);
  }
  return value;
}

function optionalWireFinite(record: StrictWireRecord, key: string, path: string): number | null | undefined {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return undefined;
  if (record[key] === null) return null;
  return wireFinite(record[key], `${path}.${key}`);
}

function wireU32(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0 || (value as number) > 0xffff_ffff) {
    return wireFail("schema.integer_invalid", path, `${path} must be an unsigned 32-bit integer.`);
  }
  return value as number;
}

function optionalWireSafeInteger(record: StrictWireRecord, key: string, path: string): number | null | undefined {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return undefined;
  if (record[key] === null) return null;
  if (!Number.isSafeInteger(record[key])) {
    return wireFail("schema.integer_invalid", `${path}.${key}`, `${path}.${key} must be a safe integer.`);
  }
  return record[key] as number;
}

function wireEnum<T extends string>(value: unknown, values: readonly T[], path: string): T {
  if (typeof value !== "string" || !values.includes(value as T)) {
    return wireFail("schema.invalid_discriminator", path, `${path} has an unsupported discriminator.`);
  }
  return value as T;
}

function validateWireCapabilityCell(value: unknown, path: string): CapabilityCellReferenceV2 {
  const cell = exactWireRecord(
    value,
    ["registry_schema_version", "capability_id", "cell_id", "capability_version"],
    [],
    path,
  );
  if (cell.registry_schema_version !== 2) {
    return wireFail("schema.version_unsupported", `${path}.registry_schema_version`, `${path}.registry_schema_version must equal 2.`);
  }
  wireStableId(cell.capability_id, `${path}.capability_id`);
  wireStableId(cell.cell_id, `${path}.cell_id`);
  wireText(cell.capability_version, `${path}.capability_version`);
  return value as CapabilityCellReferenceV2;
}

function validateCanonicalWireIds(
  values: readonly unknown[],
  key: string,
  path: string,
): string[] {
  const ids = values.map((value, index) => wireStableId(
    strictWireRecord(value, `${path}[${index}]`)[key],
    `${path}[${index}].${key}`,
  ));
  if (new Set(ids).size !== ids.length) {
    wireFail("document.invalid", path, `${path} contains duplicate stable identifiers.`);
  }
  const sorted = [...ids].sort();
  if (!ids.every((id, index) => id === sorted[index])) {
    wireFail("document.invalid", path, `${path} must be ordered by exact stable identifier.`);
  }
  return ids;
}

function validateStableIdArray(
  value: unknown,
  path: string,
  options: { minimum?: number; canonical?: boolean } = {},
): string[] {
  const values = wireArray(value, path).map((item, index) => wireStableId(item, `${path}[${index}]`));
  if (values.length < (options.minimum ?? 0)) {
    wireFail("document.invalid", path, `${path} requires at least ${options.minimum ?? 0} values.`);
  }
  if (new Set(values).size !== values.length) {
    wireFail("document.invalid", path, `${path} must not contain duplicate identifiers.`);
  }
  if (options.canonical) {
    const sorted = [...values].sort();
    if (!values.every((id, index) => id === sorted[index])) {
      wireFail("document.invalid", path, `${path} must use canonical stable-ID order.`);
    }
  }
  return values;
}

function validateGeneralSemBounds(
  lower: number | null | undefined,
  upper: number | null | undefined,
  path: string,
): void {
  if (lower != null && upper != null && lower > upper) {
    wireFail("document.invalid", path, `${path}.lower must not exceed upper.`);
  }
}

function validateGeneralSemEstimate(value: unknown, path: string): void {
  const estimate = exactWireRecord(
    value,
    ["estimate"],
    ["standard_error", "lower", "upper", "p_value"],
    path,
  );
  wireFinite(estimate.estimate, `${path}.estimate`);
  const standardError = optionalWireFinite(estimate, "standard_error", path);
  const lower = optionalWireFinite(estimate, "lower", path);
  const upper = optionalWireFinite(estimate, "upper", path);
  const pValue = optionalWireFinite(estimate, "p_value", path);
  if (standardError != null && standardError < 0) {
    wireFail("document.invalid", `${path}.standard_error`, `${path}.standard_error must be nonnegative.`);
  }
  if (pValue != null && (pValue < 0 || pValue > 1)) {
    wireFail("document.invalid", `${path}.p_value`, `${path}.p_value must be between 0 and 1.`);
  }
  validateGeneralSemBounds(lower, upper, path);
}

interface GeneralSemWireContext {
  readonly modelId: string;
  readonly capabilityIds: ReadonlySet<string>;
}

function validateGeneralSemTrace(value: unknown, path: string, context: GeneralSemWireContext): void {
  const trace = exactWireRecord(value, ["model_id", "capability_cell"], [], path);
  const modelId = wireStableId(trace.model_id, `${path}.model_id`);
  if (modelId !== context.modelId) {
    wireFail("document.invalid", `${path}.model_id`, `${path}.model_id must equal provenance.model_id.`);
  }
  const cell = validateWireCapabilityCell(trace.capability_cell, `${path}.capability_cell`);
  const identity = capabilityCellReferenceIdentityV2(cell);
  if (!context.capabilityIds.has(identity)) {
    wireFail("document.invalid", `${path}.capability_cell`, `${path}.capability_cell is not declared by the document.`);
  }
}

function approximatelyEqualGeneralSem(left: number, right: number): boolean {
  return left === right
    || Math.abs(left - right) <= Number.EPSILON * 8 * Math.max(Math.abs(left), Math.abs(right), 1);
}

function validateConditionalProbeValues(value: unknown, path: string): number[] {
  const record = strictWireRecord(value, path);
  const kind = wireEnum(record.kind, ["data_derived_mean_plus_minus_one_sd", "explicit"] as const, `${path}.kind`);
  if (kind === "data_derived_mean_plus_minus_one_sd") {
    const values = exactWireRecord(value, ["kind", "mean", "standard_deviation"], [], path);
    const mean = wireFinite(values.mean, `${path}.mean`);
    const standardDeviation = wireFinite(values.standard_deviation, `${path}.standard_deviation`);
    if (standardDeviation < 0) {
      wireFail("document.invalid", `${path}.standard_deviation`, `${path}.standard_deviation must be nonnegative.`);
    }
    return [mean - standardDeviation, mean, mean + standardDeviation];
  }
  const values = exactWireRecord(value, ["kind", "values"], [], path);
  const explicit = wireArray(values.values, `${path}.values`).map((item, index) => wireFinite(item, `${path}.values[${index}]`));
  if (explicit.length === 0) wireFail("document.invalid", `${path}.values`, `${path}.values must not be empty.`);
  for (let index = 1; index < explicit.length; index += 1) {
    if (explicit[index - 1]! >= explicit[index]!) {
      wireFail("document.invalid", `${path}.values`, `${path}.values must be strictly increasing.`);
    }
  }
  return explicit;
}

/** Strict, lossless validator for the optional Rust General SEM result extension. */
export function parseCanonicalGeneralSemResultsV1(
  value: unknown,
  context: { modelId: string; capabilityCells: readonly CapabilityCellReferenceV2[] },
): CanonicalGeneralSemResultsV1 {
  if (hasNonFiniteNumber(value)) {
    return wireFail("schema.non_finite", "general_sem_results", "general_sem_results contains a non-finite number or cyclic value.");
  }
  const results = exactWireRecord(
    value,
    ["schema_version"],
    [
      "specific_indirect_effects",
      "aggregate_effects",
      "conditional_effect_probes",
      "conditional_effects",
      "interaction_plots",
      "higher_order_stages",
      "cbsem_fit",
      "identification_diagnostics",
    ],
    "general_sem_results",
  );
  if (results.schema_version !== CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION) {
    return wireFail(
      "schema.version_unsupported",
      "general_sem_results.schema_version",
      "general_sem_results.schema_version must equal 1.",
    );
  }
  const wireContext: GeneralSemWireContext = {
    modelId: wireStableId(context.modelId, "provenance.model_id"),
    capabilityIds: new Set(context.capabilityCells.map((cell, index) => (
      capabilityCellReferenceIdentityV2(validateWireCapabilityCell(cell, `capability_cells[${index}]`))
    ))),
  };
  if (wireContext.capabilityIds.size === 0) {
    return wireFail("document.invalid", "capability_cells", "general_sem_results requires document capability_cells.");
  }

  const specific = optionalWireArray(results, "specific_indirect_effects", "general_sem_results");
  const aggregate = optionalWireArray(results, "aggregate_effects", "general_sem_results");
  const probes = optionalWireArray(results, "conditional_effect_probes", "general_sem_results");
  const conditional = optionalWireArray(results, "conditional_effects", "general_sem_results");
  const plots = optionalWireArray(results, "interaction_plots", "general_sem_results");
  const hocStages = optionalWireArray(results, "higher_order_stages", "general_sem_results");
  const fits = optionalWireArray(results, "cbsem_fit", "general_sem_results");
  const identification = optionalWireArray(results, "identification_diagnostics", "general_sem_results");
  if ([specific, aggregate, probes, conditional, plots, hocStages, fits, identification]
    .every((collection) => collection.length === 0)) {
    return wireFail("document.invalid", "general_sem_results", "general_sem_results must contain at least one typed result section.");
  }

  validateCanonicalWireIds(specific, "effect_id", "general_sem_results.specific_indirect_effects");
  validateCanonicalWireIds(aggregate, "effect_id", "general_sem_results.aggregate_effects");
  validateCanonicalWireIds(probes, "probe_id", "general_sem_results.conditional_effect_probes");
  validateCanonicalWireIds(conditional, "effect_id", "general_sem_results.conditional_effects");
  validateCanonicalWireIds(plots, "plot_id", "general_sem_results.interaction_plots");
  validateCanonicalWireIds(hocStages, "stage_id", "general_sem_results.higher_order_stages");
  validateCanonicalWireIds(fits, "fit_id", "general_sem_results.cbsem_fit");
  validateCanonicalWireIds(identification, "diagnostic_id", "general_sem_results.identification_diagnostics");

  const effectIds = new Set<string>();
  const specificSignatures = new Set<string>();
  specific.forEach((value, index) => {
    const path = `general_sem_results.specific_indirect_effects[${index}]`;
    const effect = exactWireRecord(value, ["effect_id", "estimand_id", "trace", "ordered_relation_ids", "value"], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    wireStableId(effect.estimand_id, `${path}.estimand_id`);
    validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const relations = validateStableIdArray(effect.ordered_relation_ids, `${path}.ordered_relation_ids`, { minimum: 2 });
    const signature = relations.join("\0");
    if (specificSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another specific indirect path.`);
    specificSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  const aggregateSignatures = new Set<string>();
  aggregate.forEach((value, index) => {
    const path = `general_sem_results.aggregate_effects[${index}]`;
    const effect = exactWireRecord(value, ["effect_id", "estimand_id", "trace", "kind", "source_id", "target_id", "value"], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    wireStableId(effect.estimand_id, `${path}.estimand_id`);
    validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const kind = wireEnum(effect.kind, ["total_indirect", "total_effect"] as const, `${path}.kind`);
    const sourceId = wireStableId(effect.source_id, `${path}.source_id`);
    const targetId = wireStableId(effect.target_id, `${path}.target_id`);
    if (sourceId === targetId) wireFail("document.invalid", path, `${path} requires distinct source_id and target_id.`);
    const signature = `${kind}\0${sourceId}\0${targetId}`;
    if (aggregateSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another aggregate scientific effect.`);
    aggregateSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  const probeValues = new Map<string, { moderatorId: string; values: number[] }>();
  probes.forEach((value, index) => {
    const path = `general_sem_results.conditional_effect_probes[${index}]`;
    const probe = exactWireRecord(value, ["probe_id", "trace", "moderator_id", "values"], [], path);
    const probeId = wireStableId(probe.probe_id, `${path}.probe_id`);
    validateGeneralSemTrace(probe.trace, `${path}.trace`, wireContext);
    probeValues.set(probeId, {
      moderatorId: wireStableId(probe.moderator_id, `${path}.moderator_id`),
      values: validateConditionalProbeValues(probe.values, `${path}.values`),
    });
  });

  const conditionalSignatures = new Set<string>();
  conditional.forEach((value, index) => {
    const path = `general_sem_results.conditional_effects[${index}]`;
    const effect = exactWireRecord(value, [
      "effect_id", "estimand_id", "trace", "interaction_id", "focal_relation_id", "probe_id",
      "moderator_id", "probe_value_index", "moderator_value", "value",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    const estimandId = wireStableId(effect.estimand_id, `${path}.estimand_id`);
    validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const interactionId = wireStableId(effect.interaction_id, `${path}.interaction_id`);
    const focalRelationId = wireStableId(effect.focal_relation_id, `${path}.focal_relation_id`);
    const probeId = wireStableId(effect.probe_id, `${path}.probe_id`);
    const moderatorId = wireStableId(effect.moderator_id, `${path}.moderator_id`);
    const probeValueIndex = wireU32(effect.probe_value_index, `${path}.probe_value_index`);
    const moderatorValue = wireFinite(effect.moderator_value, `${path}.moderator_value`);
    const probe = probeValues.get(probeId);
    if (!probe) wireFail("document.invalid", `${path}.probe_id`, `${path}.probe_id references a missing probe.`);
    if (probe.moderatorId !== moderatorId) wireFail("document.invalid", `${path}.moderator_id`, `${path}.moderator_id contradicts its probe.`);
    const expectedValue = probe.values[probeValueIndex];
    if (expectedValue === undefined || !approximatelyEqualGeneralSem(moderatorValue, expectedValue)) {
      wireFail("document.invalid", `${path}.moderator_value`, `${path}.moderator_value contradicts its probe value.`);
    }
    const signature = `${estimandId}\0${interactionId}\0${focalRelationId}\0${probeId}\0${probeValueIndex}`;
    if (conditionalSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another conditional scientific effect.`);
    conditionalSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  plots.forEach((value, index) => {
    const path = `general_sem_results.interaction_plots[${index}]`;
    const plot = exactWireRecord(value, [
      "plot_id", "trace", "interaction_id", "focal_relation_id", "focal_predictor_id",
      "moderator_id", "outcome_id", "series",
    ], [], path);
    wireStableId(plot.plot_id, `${path}.plot_id`);
    validateGeneralSemTrace(plot.trace, `${path}.trace`, wireContext);
    wireStableId(plot.interaction_id, `${path}.interaction_id`);
    wireStableId(plot.focal_relation_id, `${path}.focal_relation_id`);
    const focalId = wireStableId(plot.focal_predictor_id, `${path}.focal_predictor_id`);
    const moderatorId = wireStableId(plot.moderator_id, `${path}.moderator_id`);
    const outcomeId = wireStableId(plot.outcome_id, `${path}.outcome_id`);
    if (new Set([focalId, moderatorId, outcomeId]).size !== 3) {
      wireFail("document.invalid", path, `${path} requires distinct focal, moderator, and outcome identities.`);
    }
    const seriesValues = wireArray(plot.series, `${path}.series`);
    if (seriesValues.length === 0) wireFail("document.invalid", `${path}.series`, `${path}.series must not be empty.`);
    validateCanonicalWireIds(seriesValues, "series_id", `${path}.series`);
    let commonGrid: number[] | null = null;
    seriesValues.forEach((seriesValue, seriesIndex) => {
      const seriesPath = `${path}.series[${seriesIndex}]`;
      const series = exactWireRecord(seriesValue, ["series_id", "probe_id", "probe_value_index", "moderator_value", "points"], [], seriesPath);
      wireStableId(series.series_id, `${seriesPath}.series_id`);
      const probeId = wireStableId(series.probe_id, `${seriesPath}.probe_id`);
      const probeValueIndex = wireU32(series.probe_value_index, `${seriesPath}.probe_value_index`);
      const moderatorValue = wireFinite(series.moderator_value, `${seriesPath}.moderator_value`);
      const probe = probeValues.get(probeId);
      if (!probe) wireFail("document.invalid", `${seriesPath}.probe_id`, `${seriesPath}.probe_id references a missing probe.`);
      if (probe.moderatorId !== moderatorId) wireFail("document.invalid", `${seriesPath}.probe_id`, `${seriesPath}.probe_id uses a different moderator.`);
      const expectedValue = probe.values[probeValueIndex];
      if (expectedValue === undefined || !approximatelyEqualGeneralSem(moderatorValue, expectedValue)) {
        wireFail("document.invalid", `${seriesPath}.moderator_value`, `${seriesPath}.moderator_value contradicts its probe value.`);
      }
      const points = wireArray(series.points, `${seriesPath}.points`);
      if (points.length === 0) wireFail("document.invalid", `${seriesPath}.points`, `${seriesPath}.points must not be empty.`);
      const grid = points.map((pointValue, pointIndex) => {
        const pointPath = `${seriesPath}.points[${pointIndex}]`;
        const point = exactWireRecord(pointValue, ["focal_value", "predicted_value"], ["lower", "upper"], pointPath);
        const focalValue = wireFinite(point.focal_value, `${pointPath}.focal_value`);
        wireFinite(point.predicted_value, `${pointPath}.predicted_value`);
        const lower = optionalWireFinite(point, "lower", pointPath);
        const upper = optionalWireFinite(point, "upper", pointPath);
        validateGeneralSemBounds(lower, upper, pointPath);
        return focalValue;
      });
      for (let pointIndex = 1; pointIndex < grid.length; pointIndex += 1) {
        if (grid[pointIndex - 1]! >= grid[pointIndex]!) {
          wireFail("document.invalid", `${seriesPath}.points`, `${seriesPath}.points must use strictly increasing focal values.`);
        }
      }
      if (commonGrid && (commonGrid.length !== grid.length
        || commonGrid.some((expected, gridIndex) => !approximatelyEqualGeneralSem(expected, grid[gridIndex]!)))) {
        wireFail("document.invalid", `${seriesPath}.points`, `${seriesPath}.points must use the plot's common focal-value grid.`);
      }
      commonGrid ??= grid;
    });
  });

  const hocSignatures = new Set<string>();
  hocStages.forEach((value, index) => {
    const path = `general_sem_results.higher_order_stages[${index}]`;
    const stage = exactWireRecord(value, [
      "stage_id", "trace", "higher_order_construct_id", "stage_number", "kind",
      "input_construct_ids", "output_variable_ids",
    ], ["relation_estimates"], path);
    wireStableId(stage.stage_id, `${path}.stage_id`);
    validateGeneralSemTrace(stage.trace, `${path}.trace`, wireContext);
    const hocId = wireStableId(stage.higher_order_construct_id, `${path}.higher_order_construct_id`);
    const stageNumber = wireU32(stage.stage_number, `${path}.stage_number`);
    const kind = wireEnum(stage.kind, ["lower_order_score_estimation", "higher_order_estimation"] as const, `${path}.kind`);
    const expectedStage = kind === "lower_order_score_estimation" ? 1 : 2;
    if (stageNumber !== expectedStage) wireFail("document.invalid", `${path}.stage_number`, `${path}.stage_number contradicts its stage kind.`);
    const signature = `${hocId}\0${stageNumber}`;
    if (hocSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates a higher-order construct stage.`);
    hocSignatures.add(signature);
    validateStableIdArray(stage.input_construct_ids, `${path}.input_construct_ids`, { minimum: 1, canonical: true });
    validateStableIdArray(stage.output_variable_ids, `${path}.output_variable_ids`, { minimum: 1, canonical: true });
    const relations = optionalWireArray(stage, "relation_estimates", path);
    validateCanonicalWireIds(relations, "relation_id", `${path}.relation_estimates`);
    relations.forEach((relationValue, relationIndex) => {
      const relationPath = `${path}.relation_estimates[${relationIndex}]`;
      const relation = exactWireRecord(relationValue, ["relation_id", "source_id", "target_id", "value"], [], relationPath);
      wireStableId(relation.relation_id, `${relationPath}.relation_id`);
      const sourceId = wireStableId(relation.source_id, `${relationPath}.source_id`);
      const targetId = wireStableId(relation.target_id, `${relationPath}.target_id`);
      if (sourceId === targetId) wireFail("document.invalid", relationPath, `${relationPath} requires distinct source_id and target_id.`);
      validateGeneralSemEstimate(relation.value, `${relationPath}.value`);
    });
  });

  fits.forEach((value, index) => {
    const path = `general_sem_results.cbsem_fit[${index}]`;
    const fit = exactWireRecord(value, ["fit_id", "trace", "chi_square", "degrees_of_freedom"], [
      "chi_square_p_value", "rmsea", "rmsea_interval", "cfi", "tli", "srmr", "aic", "bic",
    ], path);
    wireStableId(fit.fit_id, `${path}.fit_id`);
    validateGeneralSemTrace(fit.trace, `${path}.trace`, wireContext);
    const chiSquare = wireFinite(fit.chi_square, `${path}.chi_square`);
    if (chiSquare < 0) wireFail("document.invalid", `${path}.chi_square`, `${path}.chi_square must be nonnegative.`);
    const degreesOfFreedom = wireU32(fit.degrees_of_freedom, `${path}.degrees_of_freedom`);
    const pValue = optionalWireFinite(fit, "chi_square_p_value", path);
    if (pValue != null && (pValue < 0 || pValue > 1)) wireFail("document.invalid", `${path}.chi_square_p_value`, `${path}.chi_square_p_value must be between 0 and 1.`);
    if (degreesOfFreedom === 0 && pValue != null) wireFail("document.invalid", `${path}.chi_square_p_value`, `${path}.chi_square_p_value must be absent when degrees_of_freedom is zero.`);
    const rmsea = optionalWireFinite(fit, "rmsea", path);
    const srmr = optionalWireFinite(fit, "srmr", path);
    for (const key of ["cfi", "tli", "aic", "bic"] as const) optionalWireFinite(fit, key, path);
    if (rmsea != null && rmsea < 0) wireFail("document.invalid", `${path}.rmsea`, `${path}.rmsea must be nonnegative.`);
    if (srmr != null && srmr < 0) wireFail("document.invalid", `${path}.srmr`, `${path}.srmr must be nonnegative.`);
    if (Object.prototype.hasOwnProperty.call(fit, "rmsea_interval") && fit.rmsea_interval !== null) {
      if (rmsea == null) wireFail("document.invalid", `${path}.rmsea_interval`, `${path}.rmsea_interval requires rmsea.`);
      const interval = exactWireRecord(fit.rmsea_interval, ["confidence_level", "lower", "upper"], [], `${path}.rmsea_interval`);
      const confidence = wireFinite(interval.confidence_level, `${path}.rmsea_interval.confidence_level`);
      const lower = wireFinite(interval.lower, `${path}.rmsea_interval.lower`);
      const upper = wireFinite(interval.upper, `${path}.rmsea_interval.upper`);
      if (confidence <= 0 || confidence >= 1) wireFail("document.invalid", `${path}.rmsea_interval.confidence_level`, `${path}.rmsea_interval.confidence_level must be between 0 and 1.`);
      if (lower < 0) wireFail("document.invalid", `${path}.rmsea_interval.lower`, `${path}.rmsea_interval.lower must be nonnegative.`);
      validateGeneralSemBounds(lower, upper, `${path}.rmsea_interval`);
    }
  });

  identification.forEach((value, index) => {
    const path = `general_sem_results.identification_diagnostics[${index}]`;
    const diagnostic = exactWireRecord(value, ["diagnostic_id", "trace", "scope", "subject_id", "status", "code", "message"], ["degrees_of_freedom"], path);
    wireStableId(diagnostic.diagnostic_id, `${path}.diagnostic_id`);
    validateGeneralSemTrace(diagnostic.trace, `${path}.trace`, wireContext);
    const scope = wireEnum(diagnostic.scope, ["model", "variable", "relation", "interaction", "higher_order_construct"] as const, `${path}.scope`);
    const subjectId = wireStableId(diagnostic.subject_id, `${path}.subject_id`);
    const status = wireEnum(diagnostic.status, ["identified", "underidentified", "locally_underidentified", "boundary_condition"] as const, `${path}.status`);
    wireStableId(diagnostic.code, `${path}.code`);
    wireText(diagnostic.message, `${path}.message`);
    const degreesOfFreedom = optionalWireSafeInteger(diagnostic, "degrees_of_freedom", path);
    if (scope === "model" && subjectId !== wireContext.modelId) wireFail("document.invalid", `${path}.subject_id`, `${path}.subject_id must equal provenance.model_id for model scope.`);
    if (status === "identified" && degreesOfFreedom != null && degreesOfFreedom < 0) {
      wireFail("document.invalid", `${path}.degrees_of_freedom`, `${path} cannot be identified with negative degrees_of_freedom.`);
    }
  });

  return value as CanonicalGeneralSemResultsV1;
}

/**
 * Strictly validates a complete cross-runtime document while preserving the
 * original object/key ordering for byte-stable JSON readback.
 */
export function parseNativeCanonicalResultDocumentV2(value: unknown): NativeCanonicalResultDocumentV2 {
  const document = exactWireRecord(value, [
    "schema_version", "document_id", "title", "provenance", "sections", "tables", "charts",
    "notices", "exclusions", "footnotes", "presentation",
  ], ["capability_cells", "general_sem_results"], "document");
  if (document.schema_version !== CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION) {
    return wireFail("schema.version_unsupported", "document.schema_version", `document.schema_version must equal ${CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION}.`);
  }
  if (hasNonFiniteNumber(value)) {
    return wireFail("schema.non_finite", "document", "The canonical result document contains a non-finite number or cyclic value.");
  }
  let validation;
  try {
    validation = validateCanonicalResultDocumentV2(value as CanonicalResultDocumentV2);
  } catch {
    return wireFail("schema.invalid_shape", "document", "The base CanonicalResultDocumentV2 shape is invalid.");
  }
  if (!validation.passed) {
    return wireFail("document.invalid", "document", validation.errors.join(" "));
  }
  if (Object.prototype.hasOwnProperty.call(document, "general_sem_results")) {
    const base = value as CanonicalResultDocumentV2;
    parseCanonicalGeneralSemResultsV1(document.general_sem_results, {
      modelId: base.provenance.model_id,
      capabilityCells: base.capability_cells ?? [],
    });
  }
  return value as NativeCanonicalResultDocumentV2;
}

function stableId(label: string, fallback: string): string {
  const normalized = label
    .normalize("NFKD")
    .toLowerCase()
    .replace(/[^a-z0-9_.:-]+/g, "_")
    .replace(/^[_:.-]+|[_:.-]+$/g, "")
    .replace(/_+/g, "_");
  return /^[a-z0-9]/.test(normalized) ? normalized : fallback;
}

function uniqueColumnIds(columns: readonly string[]): string[] {
  const counts = new Map<string, number>();
  return columns.map((label, index) => {
    const base = stableId(label, `column_${index + 1}`);
    const occurrence = (counts.get(base) ?? 0) + 1;
    counts.set(base, occurrence);
    return occurrence === 1 ? base : `${base}_${occurrence}`;
  });
}

function missingReason(value: string): CanonicalMissingReason | null {
  return MISSING_VALUES.get(value.trim().toLowerCase()) ?? null;
}

function columnType(values: readonly string[]): CanonicalColumnType {
  const meaningful = values.filter((value) => missingReason(value) === null);
  if (meaningful.length > 0 && meaningful.every((value) => STRICT_NUMBER.test(value.trim()))) return "number";
  if (meaningful.length > 0 && meaningful.every((value) => STRICT_BOOLEAN.test(value.trim()))) return "boolean";
  return "text";
}

function columnRole(label: string, dataType: CanonicalColumnType) {
  const normalized = label.toLowerCase();
  if (/p[- ]?value|confidence|lower|upper|standard error|t statistic|t-value/.test(normalized)) return "uncertainty" as const;
  if (/decision|significant|meets|status/.test(normalized)) return "decision" as const;
  if (/warning|failure|converged|iterations|observations/.test(normalized)) return "diagnostic" as const;
  if (dataType === "number") return "estimate" as const;
  return "label" as const;
}

function currentTable(
  table: ResultTable,
  precision: number,
  missingValueLabel: string,
): CanonicalResultTable {
  const ids = uniqueColumnIds(table.columns);
  const types = table.columns.map((_label, columnIndex) => columnType(table.rows.map((row) => row[columnIndex] ?? "")));
  return {
    id: table.id,
    title: table.title,
    description: `Runtime result table for ${table.title}.`,
    columns: table.columns.map((label, index) => ({
      id: ids[index],
      label,
      data_type: types[index],
      description: `${label} reported in ${table.title}.`,
      role: columnRole(label, types[index]),
    })),
    rows: table.rows.map((row, rowIndex) => ({
      id: `row_${rowIndex + 1}`,
      cells: row.map((raw, columnIndex): CanonicalResultCell => {
        const reason = missingReason(raw);
        if (reason) return { kind: "missing", reason, display: missingValueLabel };
        if (types[columnIndex] === "number") {
          const value = Number(raw.trim());
          return { kind: "number", value, display: value.toFixed(precision) };
        }
        if (types[columnIndex] === "boolean") return { kind: "boolean", value: raw.trim().toLowerCase() === "true" };
        return { kind: "text", value: raw };
      }),
    })),
    footnote_ids: [],
  };
}

function posthocMissingCell(missingValueLabel: string): CanonicalResultCell {
  return { kind: "missing", reason: "not_estimated", display: missingValueLabel };
}

function posthocOptionalText(value: string | null | undefined, missingValueLabel: string): CanonicalResultCell {
  return value == null ? posthocMissingCell(missingValueLabel) : { kind: "text", value };
}

function posthocOptionalNumber(value: number | null | undefined, missingValueLabel: string): CanonicalResultCell {
  return value == null ? posthocMissingCell(missingValueLabel) : { kind: "number", value };
}

function posthocOptionalBoolean(value: boolean | null | undefined, missingValueLabel: string): CanonicalResultCell {
  return value == null ? posthocMissingCell(missingValueLabel) : { kind: "boolean", value };
}

/**
 * Preserve the post-hoc result as one typed analytical row. The generic
 * two-column Result/Value table intentionally remains a presentation view;
 * using it here would coerce sample sizes and probabilities to text because
 * its Value column also contains labels.
 */
function currentPosthocTable(
  run: AnalysisRun,
  capabilityCells: CapabilityCellReferenceV2[],
  missingValueLabel: string,
): CanonicalResultTable | null {
  const result = nativePlsPosthocMinimumSampleSizeProjection(run);
  if (!result || result.method_version !== "inverse_square_root_posthoc_v2") return null;
  const column = (
    id: string,
    label: string,
    data_type: CanonicalColumnType,
    description: string,
    role: "label" | "estimate" | "uncertainty" | "decision" | "diagnostic" | "provenance",
  ) => ({ id, label, data_type, description, role });
  return {
    id: "posthoc_minimum_sample_size",
    title: "Post-hoc minimum sample size",
    description: "Retrospective inverse-square-root technical sample-size diagnostic. The formula uses a directional-test assumption; significance-aware driver selection uses the separately recorded two-sided bootstrap probability contract.",
    columns: [
      column("status", "Status", "text", "Typed availability or boundary state.", "diagnostic"),
      column("method_version", "Method version", "text", "Frozen analytical method identity.", "provenance"),
      column("formula_test", "Formula test", "text", "Test-direction assumption used by the inverse-square-root constant.", "provenance"),
      column("alpha", "Formula alpha", "number", "Significance level embedded in the inverse-square-root constant.", "provenance"),
      column("power", "Formula power", "number", "Statistical power embedded in the inverse-square-root constant.", "provenance"),
      column("constant", "Formula constant", "number", "Inverse-square-root constant.", "provenance"),
      column("selection_rule", "Driver selection", "text", "Rule used to select the path coefficient that drives the calculation.", "provenance"),
      column("significance_source", "Selection inference", "text", "Inference source used only for statistically significant path selection.", "provenance"),
      column("significance_alpha", "Selection alpha", "number", "Two-sided bootstrap probability threshold used for driver selection.", "provenance"),
      column("eligible_paths", "Eligible paths", "number", "Number of structural paths eligible for selection.", "diagnostic"),
      column("significant_paths", "Significant paths", "number", "Number of structural paths meeting the recorded selection threshold.", "diagnostic"),
      column("driver_source", "Driver source", "text", "Source construct of the selected path.", "label"),
      column("driver_target", "Driver target", "text", "Target construct of the selected path.", "label"),
      column("driver_p_two_sided", "Driver p value (two-sided)", "number", "Two-sided bootstrap probability of the selected path.", "uncertainty"),
      column("absolute_coefficient", "Absolute coefficient", "number", "Absolute magnitude of the selected path coefficient.", "estimate"),
      column("required_sample_size", "Technically required sample size", "number", "Ceiling of (2.486 divided by the absolute driver coefficient) squared.", "estimate"),
      column("analytical_sample_size", "Analytical sample size", "number", "Valid observations used by the linked PLS estimate.", "diagnostic"),
      column("meets_requirement", "Meets technical requirement", "boolean", "Whether the analytical sample is at least the calculated technical requirement.", "decision"),
      column("caution", "Interpretation caution", "text", "Required interpretation boundary for the retrospective diagnostic.", "diagnostic"),
    ],
    rows: [{
      id: "technical_minimum",
      cells: [
        { kind: "text", value: result.status },
        { kind: "text", value: result.method_version },
        { kind: "text", value: result.test },
        { kind: "number", value: result.alpha },
        { kind: "number", value: result.power },
        { kind: "number", value: result.inverse_square_root_constant },
        { kind: "text", value: result.selection_rule ?? "" },
        posthocOptionalText(result.significance_source, missingValueLabel),
        posthocOptionalNumber(result.significance_alpha, missingValueLabel),
        { kind: "number", value: result.eligible_path_count ?? run.result!.paths.length },
        posthocOptionalNumber(result.significant_path_count, missingValueLabel),
        posthocOptionalText(result.driver_source, missingValueLabel),
        posthocOptionalText(result.driver_target, missingValueLabel),
        posthocOptionalNumber(result.driver_p_value_two_sided, missingValueLabel),
        posthocOptionalNumber(result.minimum_absolute_path_coefficient, missingValueLabel),
        posthocOptionalNumber(result.technically_required_sample_size, missingValueLabel),
        { kind: "number", value: result.analytical_sample_size },
        posthocOptionalBoolean(result.meets_technical_requirement, missingValueLabel),
        { kind: "text", value: result.caution },
      ],
    }],
    footnote_ids: [],
    capability_cells: capabilityCells,
  };
}

function mergeTableSections(run: AnalysisRun, tableIds: ReadonlySet<string>): CanonicalResultSection[] {
  const navigation = buildNativeResultNavigation(run);
  const sections: CanonicalResultSection[] = [];
  const byId = new Map<string, CanonicalResultSection>();
  for (const group of navigation.groups) {
    const groupTableIds = group.items.flatMap((item) => item.kind === "table" && tableIds.has(item.tableId) ? [item.tableId] : []);
    if (groupTableIds.length === 0) continue;
    const existing = byId.get(group.id);
    if (existing) {
      for (const id of groupTableIds) if (!existing.table_ids.includes(id)) existing.table_ids.push(id);
      continue;
    }
    const section: CanonicalResultSection = {
      id: group.id,
      title: group.title,
      table_ids: [...groupTableIds],
      chart_ids: [],
    };
    byId.set(group.id, section);
    sections.push(section);
  }
  const referenced = new Set(sections.flatMap((section) => section.table_ids));
  const remaining = [...tableIds].filter((id) => !referenced.has(id));
  if (remaining.length > 0) sections.push({ id: "other_results", title: "Other results", table_ids: remaining, chart_ids: [] });
  return sections;
}

function noticeInputs(run: AnalysisRun, tables: readonly ResultTable[]) {
  const inputs: Array<{ message: string; tableId: string | null; code: string }> = [];
  for (const message of run.warnings) inputs.push({ message, tableId: null, code: "run_warning" });
  for (const message of run.result?.warnings ?? []) inputs.push({ message, tableId: null, code: "method_warning" });
  for (const message of run.assessment?.warnings ?? []) inputs.push({ message, tableId: null, code: "assessment_warning" });
  for (const table of tables) {
    if (table.warning) inputs.push({ message: table.warning, tableId: table.id, code: "table_warning" });
    if (table.status === "experimental" && !table.warning) {
      inputs.push({ message: `${table.title} is an Experimental result.`, tableId: table.id, code: "experimental_result" });
    }
  }
  const aggregated: Array<{ message: string; tableIds: string[]; code: string }> = [];
  const byMessage = new Map<string, (typeof aggregated)[number]>();
  for (const item of inputs) {
    const message = item.message.trim();
    if (!message) continue;
    const existing = byMessage.get(message);
    if (existing) {
      if (item.tableId && !existing.tableIds.includes(item.tableId)) existing.tableIds.push(item.tableId);
      continue;
    }
    const created = { message, tableIds: item.tableId ? [item.tableId] : [], code: item.code };
    byMessage.set(message, created);
    aggregated.push(created);
  }
  return aggregated;
}

async function noticesFor(
  run: AnalysisRun,
  tables: readonly ResultTable[],
  sections: readonly CanonicalResultSection[],
): Promise<CanonicalResultNotice[]> {
  const sectionByTable = new Map<string, string[]>();
  for (const section of sections) {
    for (const tableId of section.table_ids) {
      const ids = sectionByTable.get(tableId) ?? [];
      ids.push(section.id);
      sectionByTable.set(tableId, ids);
    }
  }
  return Promise.all(noticeInputs(run, tables).map(async (input, index) => ({
    id: `notice_${index + 1}_${(await sha256Hex(`${input.code}\u0000${input.tableIds.join("\u0000")}\u0000${input.message}`)).slice(0, 10)}`,
    code: input.code,
    severity: "warning" as const,
    message: input.message,
    section_ids: [...new Set(input.tableIds.flatMap((tableId) => sectionByTable.get(tableId) ?? []))],
    table_ids: input.tableIds,
  })));
}

async function legacyDatasetFingerprintNotice(
  run: AnalysisRun,
): Promise<CanonicalResultNotice | null> {
  const recorded = run.provenance?.dataset_fingerprint ?? run.fingerprint;
  if (RECORDED_DIGEST.test(recorded.trim().toLowerCase())) return null;
  const message = "This run stored a legacy dataset fingerprint identifier rather than a 64-digit SHA-256. The compatibility digest binds that identifier; it does not re-hash the raw observations.";
  return {
    id: `notice_dataset_fingerprint_${(await sha256Hex(message)).slice(0, 10)}`,
    code: "legacy_dataset_fingerprint_identifier",
    severity: "information",
    message,
    section_ids: [],
    table_ids: [],
  };
}

async function provenanceFor(
  run: AnalysisRun,
  context: NativeCanonicalResultContextV2,
  capabilityCell: CapabilityCellReferenceV2,
): Promise<CanonicalResultProvenanceV2> {
  const scientificModel = stableJson(scientificModelProjection(run));
  const modelDigest = await sha256Hex(scientificModel);
  const rawFingerprint = run.provenance?.dataset_fingerprint ?? run.fingerprint;
  const datasetFingerprint = await recordedOrDerivedDigest(rawFingerprint, "quickpls.dataset_fingerprint_identifier.v1");
  const recipeIdentity = run.provenance
    ? (() => {
        const { workers: _executionWorkers, ...analyticalSettings } = run.provenance!.settings;
        return {
          method: run.provenance!.method,
          method_version: run.provenance!.method_version,
          settings: analyticalSettings,
        };
      })()
    : {
        historical_run_id: run.id,
        method: run.method,
        method_version: run.result?.method_version ?? "historical_unrecorded",
        seed: run.seed,
      };
  const recipeDigest = await sha256Hex(stableJson(recipeIdentity));
  return {
    run_id: run.id,
    project_id: context.projectId?.trim() || "runtime-project",
    model_id: run.modelId?.trim() || `model:${modelDigest.slice(0, 20)}`,
    model_digest: modelDigest,
    dataset_id: context.datasetId?.trim() || `dataset:${datasetFingerprint.slice(0, 20)}`,
    dataset_fingerprint: datasetFingerprint,
    recipe_id: run.provenance?.recipe_id ?? `historical:${run.id}`,
    recipe_digest: recipeDigest,
    capability_cell: capabilityCell,
    method_version: run.provenance?.method_version ?? run.result?.method_version ?? "historical_unrecorded",
    engine_version: run.provenance?.engine_version ?? "historical_unrecorded",
    seed: run.provenance?.seed ?? run.seed,
    workers: run.provenance?.settings.workers ?? 1,
    started_at: run.provenance?.started_at ?? run.createdAt,
    completed_at: run.provenance?.completed_at ?? run.createdAt,
  };
}

function presentationOptions(context: NativeCanonicalResultContextV2) {
  if (context.projectId != null && (typeof context.projectId !== "string" || !context.projectId.trim())) return null;
  if (context.datasetId != null && (typeof context.datasetId !== "string" || !context.datasetId.trim())) return null;
  const precision = context.presentation?.precision ?? DEFAULT_PRECISION;
  const rawMissingValueLabel = context.presentation?.missingValueLabel;
  if (rawMissingValueLabel != null && (typeof rawMissingValueLabel !== "string" || !rawMissingValueLabel.trim())) return null;
  const missingValueLabel = rawMissingValueLabel?.trim() || DEFAULT_MISSING_VALUE_LABEL;
  if (!Number.isInteger(precision) || precision < 0 || precision > 12) return null;
  const rawDefaults = context.presentation?.chartDefaults;
  if (rawDefaults != null && (typeof rawDefaults !== "object" || Array.isArray(rawDefaults))) return null;
  const chartDefaults: CanonicalChartDisplayOptions = {};
  if (rawDefaults?.palette != null) {
    if (typeof rawDefaults.palette !== "string" || !rawDefaults.palette.trim()) return null;
    chartDefaults.palette = rawDefaults.palette;
  }
  for (const key of ["show_legend", "show_values"] as const) {
    const value = rawDefaults?.[key];
    if (value != null && typeof value !== "boolean") return null;
    if (value != null) chartDefaults[key] = value;
  }
  for (const key of ["x_axis_label", "y_axis_label"] as const) {
    const value = rawDefaults?.[key];
    if (value != null && typeof value !== "string") return null;
    if (value !== undefined) chartDefaults[key] = value;
  }
  return {
    precision,
    missingValueLabel,
    chartDefaults,
  };
}

/**
 * Build the V2 runtime compatibility document for one completed result.
 *
 * Current runs receive conservative typed cells from their fail-closed native
 * tables. Runs that predate technical provenance remain readable through the
 * text-only migration path; formatted historical values are never retyped.
 */
async function buildCanonicalResultDocumentFromAnalysisRunV2(
  run: AnalysisRun | null | undefined,
  context: NativeCanonicalResultContextV2 = {},
): Promise<NativeCanonicalResultBuildV2> {
  if (!isCompletedResultRun(run)) {
    return { ok: false, code: "run_not_completed", errors: ["A completed result-backed run is required."] };
  }
  if (hasNonFiniteNumber(run.result) || hasNonFiniteNumber(run.assessment) || hasNonFiniteNumber(run.bootstrap) || hasNonFiniteNumber(run.permutation)) {
    return { ok: false, code: "invalid_analytical_payload", errors: ["The analytical payload contains a non-finite number or a cyclic value."] };
  }

  const navigation = buildNativeResultNavigation(run);
  const historical = !run.provenance;
  const legacyTables = historical
    ? []
    : navigation.tables.filter((table) => table.id === "blindfolding");
  const tables = historical
    ? navigation.tables
    : navigation.tables.filter((table) => table.id !== "blindfolding");
  const tableIds = tables.map((table) => table.id);
  if (
    navigation.runId !== run.id
    || tables.length === 0
    || new Set(tableIds).size !== tableIds.length
    || tables.some((table) => (
      !table.id.trim()
      || !table.title.trim()
      || table.columns.length === 0
      || table.rows.some((row) => row.length !== table.columns.length || row.some((value) => typeof value !== "string"))
    ))
  ) {
    return { ok: false, code: "invalid_native_result_tables", errors: ["The run did not produce one coherent native result-table set."] };
  }

  const capabilityCell = resolveCapabilityCell(run);
  if (!capabilityCell) {
    return { ok: false, code: "unresolved_capability_cell", errors: ["The run cannot be linked to one exact Capability Registry V2 cell."] };
  }
  const legacyBlindfoldingCell = legacyTables.length > 0
    ? capabilityCellForRequirement(requirement(
        "smartpls.blindfolding",
        "qpls3.assessment.blindfolding_legacy",
        "historical_blindfolding",
      ))
    : null;
  if (legacyTables.length > 0 && !legacyBlindfoldingCell) {
    return { ok: false, code: "unresolved_capability_cell", errors: ["The historical blindfolding table cannot be linked to its Legacy registry cell."] };
  }
  const presentation = presentationOptions(context);
  if (!presentation) {
    return { ok: false, code: "invalid_canonical_document", errors: ["Result precision must be an integer from 0 to 12."] };
  }

  const provenance = await provenanceFor(run, context, capabilityCell);
  const documentId = `result_document:${(await sha256Hex(`${run.id}\u0000${provenance.recipe_digest}`)).slice(0, 24)}`;
  let document: CanonicalResultDocumentV2;
  if (historical) {
    document = canonicalResultDocumentFromLegacyTables(
      { document_id: documentId, title: run.name, provenance },
      tables.map((table) => ({ ...table, warning: null })),
    );
    document.notices.push(...await noticesFor(run, tables, document.sections));
    document.presentation = {
      ...document.presentation,
      precision: presentation.precision,
      missing_value_label: presentation.missingValueLabel,
      chart_defaults: presentation.chartDefaults,
    };
  } else {
    const canonicalTables = tables.map((table) => {
      const capabilityCells = capabilityCellsForTable(run, table.id);
      if (!capabilityCells) return null;
      if (table.id === "posthoc_minimum_sample_size") {
        return currentPosthocTable(run, capabilityCells, presentation.missingValueLabel);
      }
      return {
        ...currentTable(table, presentation.precision, presentation.missingValueLabel),
        capability_cells: capabilityCells,
      };
    });
    if (canonicalTables.some((table) => table === null)) {
      return { ok: false, code: "unresolved_capability_cell", errors: ["One or more result tables cannot be linked to exact Capability Registry V2 cells."] };
    }
    const attributedTables = canonicalTables as CanonicalResultTable[];
    const tableById = new Map(attributedTables.map((table) => [table.id, table]));
    const sections = mergeTableSections(run, new Set(tableIds)).map((section) => ({
      ...section,
      capability_cells: sortedDistinctCapabilityCells(section.table_ids.flatMap((tableId) => (
        tableById.get(tableId)?.capability_cells ?? []
      ))),
    }));
    const capabilityCells = sortedDistinctCapabilityCells([
      capabilityCell,
      ...attributedTables.flatMap((table) => table.capability_cells ?? []),
    ]);
    document = {
      schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
      document_id: documentId,
      title: run.name,
      provenance,
      capability_cells: capabilityCells,
      sections,
      tables: attributedTables,
      charts: [],
      notices: await noticesFor(run, tables, sections),
      exclusions: legacyTables.map(() => ({
        id: "historical_blindfolding_omitted",
        capability_cell: legacyBlindfoldingCell!,
        title: "Historical blindfolding result omitted",
        reason: "Blindfolding is retained only when opening historical results and is not included in a current canonical report.",
      })),
      footnotes: [],
      presentation: {
        default_section_id: sections.find((section) => section.table_ids.includes(navigation.defaultItemId ?? ""))?.id
          ?? sections[0]?.id
          ?? null,
        default_table_id: tableIds.includes(navigation.defaultItemId ?? "")
          ? navigation.defaultItemId
          : attributedTables[0]?.id ?? null,
        precision: presentation.precision,
        missing_value_label: presentation.missingValueLabel,
        chart_defaults: presentation.chartDefaults,
      },
    };
  }
  const fingerprintNotice = await legacyDatasetFingerprintNotice(run);
  if (fingerprintNotice) document.notices.push(fingerprintNotice);

  const validation = validateCanonicalResultDocumentV2(document);
  if (!validation.passed) {
    return { ok: false, code: "invalid_canonical_document", errors: validation.errors };
  }
  return {
    ok: true,
    mode: historical ? "historical_text_fallback" : "current_typed_bridge",
    document,
  };
}

export async function canonicalResultDocumentFromAnalysisRunV2(
  run: AnalysisRun | null | undefined,
  context: NativeCanonicalResultContextV2 = {},
): Promise<NativeCanonicalResultBuildV2> {
  try {
    return await buildCanonicalResultDocumentFromAnalysisRunV2(run, context);
  } catch {
    return {
      ok: false,
      code: "technical_provenance_unavailable",
      errors: ["QuickPLS could not create deterministic technical provenance for this result."],
    };
  }
}
