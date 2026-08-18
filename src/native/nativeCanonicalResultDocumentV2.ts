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

export type NativeCanonicalResultBuildV2 =
  | {
      ok: true;
      mode: NativeCanonicalResultModeV2;
      document: CanonicalResultDocumentV2;
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
