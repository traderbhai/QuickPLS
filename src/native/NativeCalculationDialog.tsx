import {
  Calculator,
  ChartScatter,
  CheckCircle2,
  LoaderCircle,
  Play,
  RotateCcw,
  Scale,
  Search,
  Shuffle,
  Target,
  UsersRound,
  type LucideIcon,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ConstructData, Dataset, DatasetGroupProfile, RunMonitorState } from "../types";
import {
  capabilityCellSessionKey,
  shouldShowExperimentalWarning,
  type CapabilitySurfaceCellV2,
} from "../domain/capabilitySurfaceV2";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import {
  resolveUnifiedSemCalculationV1,
  unifiedSemModeratedMediationCandidatesV1,
  type UnifiedSemCalculationActionV1,
  type UnifiedSemCalculationContextV1,
  type UnifiedSemCalculationMethodV1,
  type UnifiedSemCalculationPlanV1,
  type UnifiedSemHigherOrderFeatureV1,
  type UnifiedSemInferenceChoiceV1,
  type UnifiedSemInteractionFeatureV1,
} from "../domain/unifiedSemCalculationV1";
import {
  methodCapabilityAvailabilityV2,
  methodCapabilityRequirementsV2,
  type MethodCapabilityAvailabilityV2,
  type MethodCapabilityRegistryReaderV2,
} from "../domain/methodCapabilityRegistryV2";
import { isNativeDesktop, profileNativeDatasetGroups } from "../services/projectService";
import {
  filterNativeAnalysisCatalog,
  NATIVE_ANALYSIS_CATALOG,
  nativeCapabilitySettingsForWorkbenchKindV2,
  nativeAnalysisSettingsForWorkbenchKind,
  nativeAnalysisCatalogItem,
  nativeAnalysisStartLabel,
  isNativeEstablishedWorkingAnalysisKindV1,
  type NativeAnalysisCatalogItem,
  type NativeAnalysisCategoryId,
  type NativeWorkbenchAnalysisKind,
} from "./nativeAnalysisCatalog";
import { NATIVE_ANALYSIS_RECIPE_BOUNDS, NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS } from "./nativeAnalysisRecipe";
import { buildNativePlsSampleSizePowerRecipe } from "./nativePlsSampleSizePower";
import { nativeCalculationPhaseLabel } from "./nativeCalculationLifecycle";
import type { NativePlsReadiness } from "./nativePlsReadiness";
import {
  nativeEligibleGroupColumns,
  nativeGroupOptionLabel,
  nativeMgaProfileAssessment,
  residentDatasetGroupProfile,
  type NativeMgaProfileAssessment,
} from "./nativeMga";
import { nativeIpmaTargetOptions } from "./nativeIpma";
import {
  NATIVE_NCA_DEFAULT_PERMUTATIONS,
  NATIVE_NCA_MAX_PERMUTATIONS,
  NATIVE_NCA_MIN_PERMUTATIONS,
  nativeNcaNumericColumns,
} from "./nativeNca";
import {
  NATIVE_PCA_MAX_VARIABLES,
  nativePcaNumericColumns,
  nativePcaSelectedVariables,
} from "./nativePca";
import {
  NATIVE_OLS_MAX_TERMS,
  nativeOlsCsvValues,
  nativeOlsNumericColumns,
} from "./nativeOls";
import {
  NATIVE_LOGISTIC_MAX_TERMS,
  nativeLogisticReadiness,
  profileNativeLogisticDataset,
  residentNativeLogisticProfile,
  type NativeLogisticProfile,
  type NativeLogisticReadinessAssessment,
} from "./nativeLogistic";
import { NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS } from "./nativeRegressionBootstrapWitness";
import { nativeCtaPlsEligibleBlocks } from "./nativeCtaPls";
import NativeProcessSetup from "./NativeProcessSetup";
import {
  nativeProcessReadiness,
  nativeProcessSelectionToken,
  profileNativeProcessDataset,
  residentNativeProcessProfile,
  type NativeProcessProfile,
  type NativeProcessReadinessAssessment,
} from "./nativeProcess";

export interface NativeCalculationDialogProps {
  kind: NativeWorkbenchAnalysisKind;
  setKind: (kind: NativeWorkbenchAnalysisKind) => void;
  settings: AnalysisUiSettings;
  setSettings: (patch: Partial<AnalysisUiSettings>) => void;
  readiness: NativePlsReadiness;
  runMonitor: RunMonitorState;
  dataset: Dataset;
  analysisColumns: string[];
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  start: (dataProfile?: NativeLogisticProfile | NativeProcessProfile) => void;
  cancel: () => void;
  close: () => void;
  experimentalLabsEnabled?: boolean;
  openMethodDetails?: () => void;
  /** Optional strict authority bridge. Omission preserves the existing legacy workflow. */
  unifiedSem?: UnifiedSemCalculationContextV1 | null;
  /** One event seam for strict execution and its two calculation-time editors. */
  onUnifiedSemAction?: (action: UnifiedSemCalculationActionV1) => void;
  registryUnavailableReason?: string | null;
  capabilityRegistry?: MethodCapabilityRegistryReaderV2;
}

const ACTIVE_RUN_STATUSES = new Set<RunMonitorState["status"]>([
  "queued",
  "validating",
  "running",
  "cancelling",
]);

const RETRY_RUN_STATUSES = new Set<RunMonitorState["status"]>([
  "failed",
  "cancelled",
]);

const CATEGORY_ORDER: readonly NativeAnalysisCategoryId[] = [
  "estimation",
  "component_models",
  "assessment",
  "covariance",
  "inference",
  "groups",
  "prediction",
  "standalone",
];

const METHOD_ICONS: Record<NativeWorkbenchAnalysisKind, LucideIcon> = {
  pls_algorithm: Calculator,
  plsc: CheckCircle2,
  plsc_bootstrap: RotateCcw,
  wpls: Scale,
  gsca: Calculator,
  cca: Search,
  cta_pls: Search,
  ipma: Target,
  cbsem: Calculator,
  pls_bootstrap: RotateCcw,
  pls_permutation: Shuffle,
  pls_posthoc_technical_minimum_sample_size: ChartScatter,
  pls_sample_size_power: ChartScatter,
  mga: UsersRound,
  predict: Target,
  nca: ChartScatter,
  pca: Calculator,
  regression: ChartScatter,
};

type GroupProfileState =
  | { status: "idle" | "loading"; profile: null; error: null }
  | { status: "ready"; profile: DatasetGroupProfile; error: null }
  | { status: "failed"; profile: null; error: string };

type LogisticProfileState =
  | { status: "idle" | "loading"; key: string; profile: null; error: null }
  | { status: "ready"; key: string; profile: NativeLogisticProfile; error: null }
  | { status: "failed"; key: string; profile: null; error: string };

type ProcessProfileState =
  | { status: "idle" | "loading"; key: string; profile: null; error: null }
  | { status: "ready"; key: string; profile: NativeProcessProfile; error: null }
  | { status: "failed"; key: string; profile: null; error: string };

export function shouldStartNativeProcessProfile(
  selected: boolean,
  residentProfile: NativeProcessProfile | null,
  assessmentCanRun: boolean | undefined,
  profileKey: string,
  state: Pick<ProcessProfileState, "status" | "key">,
): boolean {
  if (!selected || residentProfile || !assessmentCanRun) return false;
  return state.key !== profileKey || state.status === "idle";
}

export function retryNativeProcessProfileState(profileKey: string): ProcessProfileState {
  return { status: "idle", key: profileKey, profile: null, error: null };
}

export const NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS = {
  bootstrap: {
    min: NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.minimum,
    max: NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.maximum,
    step: 1,
  },
  permutation: {
    min: NATIVE_ANALYSIS_RECIPE_BOUNDS.permutationSamples.minimum,
    max: NATIVE_ANALYSIS_RECIPE_BOUNDS.permutationSamples.maximum,
    step: 1,
  },
} as const;

export function nativeRegressionTypeSettingsPatch(
  regressionType: "ols" | "logistic" | "process",
): Partial<AnalysisUiSettings> {
  return {
    regressionType,
    robustSe: regressionType === "logistic" ? "none" : "hc3",
    ...(regressionType === "process" ? { regressionPredictors: null } : {}),
    preprocessing: "unstandardized",
    confidenceLevel: 0.95,
    studentizedInnerSamples: 0,
    permutationSamples: 0,
  };
}

const optionId = (kind: NativeWorkbenchAnalysisKind) => `nd-calculation-method-${kind}`;
const panelTitleId = (kind: NativeWorkbenchAnalysisKind) => `nd-calculation-panel-${kind}-title`;

function unifiedSemMethodV1(kind: NativeWorkbenchAnalysisKind): UnifiedSemCalculationMethodV1 | null {
  return kind === "pls_algorithm" || kind === "pls_bootstrap" || kind === "cbsem" ? kind : null;
}

const UNIFIED_HOC_APPROACH_LABELS_V1: Readonly<Record<UnifiedSemHigherOrderFeatureV1["approach"], string>> = {
  repeated_indicators: "repeated indicators",
  extended_repeated_indicators: "extended repeated indicators",
  embedded_two_stage: "embedded two-stage",
  disjoint_two_stage: "disjoint two-stage",
  hybrid: "hybrid",
};

const UNIFIED_HOC_TYPE_CODES_V1: Readonly<Record<UnifiedSemHigherOrderFeatureV1["measurementType"], string>> = {
  reflective_reflective: "RR",
  reflective_formative: "RF",
  formative_reflective: "FR",
  formative_formative: "FF",
};

function unifiedHigherOrderSummaryV1(feature: UnifiedSemHigherOrderFeatureV1): string {
  return `${feature.label} · ${UNIFIED_HOC_TYPE_CODES_V1[feature.measurementType]} · ${UNIFIED_HOC_APPROACH_LABELS_V1[feature.approach]}`;
}

export function unifiedInteractionSummaryV1(feature: UnifiedSemInteractionFeatureV1): string {
  if (feature.order === "three_way") {
    return `${feature.moderatorLabels[1] ?? feature.moderatorIds[1]} extends ${feature.predictorLabel} × ${feature.moderatorLabels[0] ?? feature.moderatorIds[0]} → ${feature.outcomeLabel}`;
  }
  return `${feature.moderatorLabels[0] ?? feature.moderatorIds[0]} moderates ${feature.predictorLabel} → ${feature.outcomeLabel}`;
}

export interface NativeCalculationCatalogEntryV2 {
  readonly item: NativeAnalysisCatalogItem;
  readonly availability: MethodCapabilityAvailabilityV2;
  readonly experimentalWarningCells: readonly CapabilitySurfaceCellV2[];
}

function nativeExperimentalWarningCellsV2(
  settings: Readonly<AnalysisUiSettings>,
  registry: MethodCapabilityRegistryReaderV2 = capabilityRegistryV2,
): readonly CapabilitySurfaceCellV2[] {
  try {
    return methodCapabilityRequirementsV2(settings).flatMap((required) => {
      const match = registry.quickPlsCell(required.cell_id).find((candidate) => (
        candidate.row.capability_id === required.capability_id
        && candidate.cell.capability_id === required.capability_id
        && candidate.cell.cell_id === required.cell_id
        && candidate.link.capability_id === required.capability_id
        && candidate.link.cell_id === required.cell_id
      ));
      if (!match) return [];
      return [{
        capability_id: match.cell.capability_id,
        cell_id: match.cell.cell_id,
        capability_version: match.cell.capability_version,
        coverage_state: match.cell.coverage_state,
        evidence_state: match.cell.evidence_state,
        surface: match.cell.surface,
      }];
    });
  } catch {
    return [];
  }
}

function nativeCalculationMethodIsVisible(
  entry: NativeCalculationCatalogEntryV2,
  experimentalLabsEnabled: boolean,
): boolean {
  const { availability, item } = entry;
  return (availability.selectable
      && (availability.tier === "standard"
        || (experimentalLabsEnabled && availability.tier === "experimental")))
    || (experimentalLabsEnabled && isNativeEstablishedWorkingAnalysisKindV1(item.kind));
}

/**
 * Resolve every native catalog kind through its concrete normalized recipe.
 * The registry bridge is deliberately fail-closed for missing methods,
 * unknown options, and unavailable add-ons.
 */
export function nativeCalculationCatalogEntriesV2(
  settings: Readonly<AnalysisUiSettings>,
  experimentalLabsEnabled: boolean,
  registry: MethodCapabilityRegistryReaderV2 = capabilityRegistryV2,
): readonly NativeCalculationCatalogEntryV2[] {
  return NATIVE_ANALYSIS_CATALOG.map((item) => {
    const capabilitySettings = nativeCapabilitySettingsForWorkbenchKindV2(settings, item.kind);
    return {
      item,
      availability: methodCapabilityAvailabilityV2(capabilitySettings, { experimentalLabsEnabled, registry }),
      experimentalWarningCells: nativeExperimentalWarningCellsV2(capabilitySettings, registry),
    };
  });
}

export function nativeVisibleCalculationCatalogV2(
  settings: Readonly<AnalysisUiSettings>,
  experimentalLabsEnabled: boolean,
  query = "",
  registry: MethodCapabilityRegistryReaderV2 = capabilityRegistryV2,
): readonly NativeCalculationCatalogEntryV2[] {
  const searchMatches = new Set(filterNativeAnalysisCatalog(query).map((item) => item.kind));
  return nativeCalculationCatalogEntriesV2(settings, experimentalLabsEnabled, registry).filter((entry) => (
    searchMatches.has(entry.item.kind)
    && nativeCalculationMethodIsVisible(entry, experimentalLabsEnabled)
  ));
}

export function nativeExperimentalWarningSessionKeys(
  entry: NativeCalculationCatalogEntryV2 | null | undefined,
  experimentalLabsEnabled: boolean,
  shownThisSession: ReadonlySet<string>,
): readonly string[] {
  if (!entry?.availability.selectable || entry.availability.tier !== "experimental") return [];
  return entry.experimentalWarningCells
    .filter((cell) => shouldShowExperimentalWarning(cell, experimentalLabsEnabled, shownThisSession))
    .map((cell) => capabilityCellSessionKey(cell));
}

export function scrollNativeMethodOptionIntoView(
  option: Pick<HTMLButtonElement, "scrollIntoView"> | null | undefined,
) {
  option?.scrollIntoView({ behavior: "auto", block: "nearest", inline: "nearest" });
}

export function dispatchNativeCalculationStartV1<T>(
  plan: UnifiedSemCalculationPlanV1 | null,
  onUnifiedSemAction: ((action: UnifiedSemCalculationActionV1) => void) | undefined,
  legacyStart: (profile?: T) => void,
  profile?: T,
): "unified_sem" | "legacy" | "unavailable" {
  if (plan && plan.route !== "legacy") {
    if (!onUnifiedSemAction) return "unavailable";
    onUnifiedSemAction({ kind: "start", plan });
    return "unified_sem";
  }
  legacyStart(profile);
  return "legacy";
}

export interface NativeCalculationBlockingMessageV1 {
  readonly cause: string;
  readonly correction: string | null;
}

export function unifiedSemPrimaryBlockingMessageV1(
  plan: UnifiedSemCalculationPlanV1 | null | undefined,
): NativeCalculationBlockingMessageV1 | null {
  const diagnostic = plan?.decision?.diagnostics.find((item) => item.severity === "error");
  if (diagnostic) return {
    cause: diagnostic.message,
    correction: diagnostic.corrections[0] ?? null,
  };
  const cause = plan?.blockers[0]?.trim();
  return cause ? { cause, correction: null } : null;
}

/**
 * Returns only variables that are declared numeric, or that can be safely
 * inferred as numeric from every resident non-missing value when metadata is
 * absent. The native runtime still validates the complete column before WPLS.
 */
export function nativeNumericCaseWeightColumns(dataset: Readonly<Dataset>): string[] {
  const metadata = new Map((dataset.columnMetadata ?? []).map((column) => [column.name, column]));
  return dataset.columns.filter((column) => {
    const declared = metadata.get(column);
    if (declared) return declared.column_type === "numeric";

    const values = dataset.rows
      .map((row) => row[column])
      .filter((value): value is string | number => value !== null && value !== undefined && value !== "");
    return values.length > 0 && values.every((value) => typeof value === "number" && Number.isFinite(value));
  });
}

export default function NativeCalculationDialog({
  kind,
  setKind,
  settings,
  setSettings,
  readiness,
  runMonitor,
  dataset,
  analysisColumns,
  nodes,
  edges,
  start,
  cancel,
  close,
  experimentalLabsEnabled = false,
  openMethodDetails,
  unifiedSem = null,
  onUnifiedSemAction,
  registryUnavailableReason = null,
  capabilityRegistry = capabilityRegistryV2,
}: NativeCalculationDialogProps) {
  const [query, setQuery] = useState("");
  const [focusedKind, setFocusedKind] = useState<NativeWorkbenchAnalysisKind>(kind);
  const [groupProfileState, setGroupProfileState] = useState<GroupProfileState>({ status: "idle", profile: null, error: null });
  const [logisticProfileState, setLogisticProfileState] = useState<LogisticProfileState>({ status: "idle", key: "", profile: null, error: null });
  const [processProfileState, setProcessProfileState] = useState<ProcessProfileState>({ status: "idle", key: "", profile: null, error: null });
  const [processProfileRetryNonce, setProcessProfileRetryNonce] = useState(0);
  const [cbsemInference, setCbsemInference] = useState<UnifiedSemInferenceChoiceV1>(() => (
    unifiedSem?.config.inference.kind === "case_bootstrap" ? "case_bootstrap" : "point"
  ));
  const [moderatedMediationSelection, setModeratedMediationSelection] = useState<{
    authorityKey: string | null;
    pathId: string | null | undefined;
  }>(() => ({ authorityKey: unifiedSem?.authorityKey ?? null, pathId: undefined }));
  const searchRef = useRef<HTMLInputElement>(null);
  const optionRefs = useRef<Partial<Record<NativeWorkbenchAnalysisKind, HTMLButtonElement | null>>>({});
  const catalogEntries = useMemo(
    () => registryUnavailableReason ? [] : nativeCalculationCatalogEntriesV2(
      settings,
      experimentalLabsEnabled,
      capabilityRegistry,
    ),
    [capabilityRegistry, experimentalLabsEnabled, registryUnavailableReason, settings],
  );
  const catalogEntryByKind = useMemo(
    () => new Map(catalogEntries.map((entry) => [entry.item.kind, entry])),
    [catalogEntries],
  );
  const filteredMethods = useMemo(() => {
    const searchMatches = new Set(filterNativeAnalysisCatalog(query).map((item) => item.kind));
    return catalogEntries
      .filter((entry) => (
        searchMatches.has(entry.item.kind)
        && nativeCalculationMethodIsVisible(entry, experimentalLabsEnabled)
      ))
      .map((entry) => entry.item);
  }, [catalogEntries, experimentalLabsEnabled, query]);
  const selectedMethod = nativeAnalysisCatalogItem(kind);
  const selectedCatalogEntry = catalogEntryByKind.get(kind);
  const selectedMethodVisible = Boolean(
    selectedCatalogEntry
    && nativeCalculationMethodIsVisible(selectedCatalogEntry, experimentalLabsEnabled),
  );
  const running = ACTIVE_RUN_STATUSES.has(runMonitor.status);
  const retry = RETRY_RUN_STATUSES.has(runMonitor.status);
  const rovingKind = filteredMethods.some((method) => method.kind === focusedKind)
    ? focusedKind
    : filteredMethods.some((method) => method.kind === kind)
      ? kind
      : filteredMethods[0]?.kind;
  const groupColumn = selectedMethodVisible && kind === "mga" ? settings.groupColumn?.trim() ?? "" : "";
  const analysisColumnKey = useMemo(
    () => [...new Set(analysisColumns)].sort().join("\u0000"),
    [analysisColumns],
  );
  const stableAnalysisColumns = useMemo(
    () => analysisColumnKey ? analysisColumnKey.split("\u0000") : [],
    [analysisColumnKey],
  );
  const unifiedMethod = unifiedSemMethodV1(kind);
  const unifiedBootstrapOptions = useMemo(() => ({
    resamples: kind === "cbsem"
      ? Math.min(10_000, Math.max(500, settings.cbsemBootstrapSamples || 500))
      : Math.min(
          NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap.max,
          Math.max(
            NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap.min,
            settings.bootstrapSamples || NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.default,
          ),
        ),
    seed: settings.seed,
    confidenceLevel: kind === "cbsem" ? 0.95 : settings.confidenceLevel,
  }), [kind, settings.bootstrapSamples, settings.cbsemBootstrapSamples, settings.confidenceLevel, settings.seed]);
  const unifiedSemPlan = useMemo(() => unifiedMethod
    ? resolveUnifiedSemCalculationV1({
        method: unifiedMethod,
        context: unifiedSem,
        cbsemInference,
        moderatedMediationPathId: moderatedMediationSelection.authorityKey === unifiedSem?.authorityKey
          ? moderatedMediationSelection.pathId
          : undefined,
        bootstrap: unifiedBootstrapOptions,
      })
    : null, [cbsemInference, moderatedMediationSelection, unifiedBootstrapOptions, unifiedMethod, unifiedSem]);
  const ipmaTargetOptions = useMemo(() => nativeIpmaTargetOptions(nodes, edges), [edges, nodes]);
  const groupProfileAssessment = useMemo(
    () => nativeMgaProfileAssessment(groupProfileState.profile, settings),
    [groupProfileState.profile, settings],
  );
  const logisticSelected = selectedMethodVisible && kind === "regression" && settings.regressionType === "logistic";
  const processSelected = selectedMethodVisible && kind === "regression" && settings.regressionType === "process";
  const logisticProfileKey = useMemo(() => [
    dataset.id,
    dataset.fingerprint ?? "",
    String(dataset.rowCount ?? dataset.rows.length),
    settings.regressionOutcome?.trim() ?? "",
    settings.regressionPredictors ?? "",
    settings.regressionControls ?? "",
  ].join("\u0000"), [
    dataset.fingerprint,
    dataset.id,
    dataset.rowCount,
    dataset.rows.length,
    settings.regressionControls,
    settings.regressionOutcome,
    settings.regressionPredictors,
  ]);
  const residentLogisticProfile = useMemo(
    () => logisticSelected ? residentNativeLogisticProfile(dataset, settings) : null,
    [dataset, logisticProfileKey, logisticSelected, settings],
  );
  const currentLogisticProfileState: LogisticProfileState = residentLogisticProfile
    ? { status: "ready", key: logisticProfileKey, profile: residentLogisticProfile, error: null }
    : logisticProfileState.key === logisticProfileKey
      ? logisticProfileState
      : { status: "idle", key: logisticProfileKey, profile: null, error: null };
  const logisticProfileAssessment = useMemo(
    () => logisticSelected
      ? nativeLogisticReadiness(dataset, settings, currentLogisticProfileState.status === "ready" ? currentLogisticProfileState.profile : null)
      : null,
    [currentLogisticProfileState, dataset, logisticSelected, settings],
  );
  const processProfileKey = useMemo(() => {
    if (!processSelected) return "";
    const structure = nativeProcessReadiness(dataset, settings);
    return [
      dataset.id,
      dataset.fingerprint ?? "",
      String(dataset.rowCount ?? dataset.rows.length),
      nativeProcessSelectionToken(settings, structure),
    ].join("\u0000");
  }, [dataset, processSelected, settings]);
  // Runtime-only settings such as seed, workers, and bootstrap samples do not
  // affect the complete-case/binary profile. Retain the exact settings object
  // associated with the scientific selection key so those edits cannot cause
  // another full paged scan.
  const processProfileSettings = useMemo(() => settings, [processProfileKey]);
  const residentProcessProfile = useMemo(
    () => processSelected ? residentNativeProcessProfile(dataset, processProfileSettings) : null,
    [dataset, processProfileKey, processSelected, processProfileSettings],
  );
  const currentProcessProfileState: ProcessProfileState = residentProcessProfile
    ? { status: "ready", key: processProfileKey, profile: residentProcessProfile, error: null }
    : processProfileState.key === processProfileKey
      ? processProfileState
      : { status: "idle", key: processProfileKey, profile: null, error: null };
  const processProfileAssessment = useMemo(
    () => processSelected
      ? nativeProcessReadiness(dataset, processProfileSettings, currentProcessProfileState.status === "ready" ? currentProcessProfileState.profile : null)
      : null,
    [currentProcessProfileState, dataset, processSelected, processProfileSettings],
  );
  const groupProfileBlockers = kind !== "mga"
    ? []
    : groupProfileState.status === "loading"
      ? ["Loading complete-dataset group counts."]
      : groupProfileState.status === "failed"
        ? [groupProfileState.error]
        : groupProfileState.status === "idle"
          ? ["Choose a grouping variable to load complete-dataset counts."]
          : groupProfileAssessment.blockers;
  const logisticProfileBlockers = !logisticSelected
    ? []
    : logisticProfileAssessment && !logisticProfileAssessment.canRun
      ? logisticProfileAssessment.blockers
      : currentLogisticProfileState.status === "failed"
      ? [currentLogisticProfileState.error]
      : currentLogisticProfileState.status !== "ready"
        ? ["Profile all dataset rows before starting binary logistic regression."]
        : [];
  const processProfileBlockers = !processSelected
    ? []
    : processProfileAssessment && !processProfileAssessment.canRun
      ? processProfileAssessment.blockers
      : currentProcessProfileState.status === "failed"
        ? [currentProcessProfileState.error]
        : currentProcessProfileState.status !== "ready"
          ? ["Profile all dataset rows before starting graph-defined path analysis."]
          : [];
  const archivedCbsemBootstrapSetting = !unifiedSem
    && kind === "cbsem"
    && (settings.cbsemBootstrapSamples ?? 0) > 0;
  const cbsemPointRouteBlockers = archivedCbsemBootstrapSetting
    ? ["Clear the archived bootstrap setting before running this legacy point-estimate setup. Reopen the project through the unified CB-SEM calculation workflow for saved bootstrap inference."]
    : [];
  const unifiedRouteSelected = Boolean(unifiedSemPlan && unifiedSemPlan.route !== "legacy");
  const unifiedReadinessBlockers = unifiedRouteSelected
    ? unifiedSemPlan?.controllerPreflightRequired
      // The exact-CB compatibility controller owns dataset/model eligibility,
      // including covariance and correlation input. Retain only the shared
      // desktop-runtime gate here so PLS-specific raw-data blockers cannot
      // prevent that controller from opening.
      ? readiness.blockers.filter((blocker) => blocker.id === "runtime")
      : readiness.blockers.filter((blocker) => blocker.id !== "calculation")
    : [];
  const methodProfileBlockers = [...new Set([
    ...groupProfileBlockers,
    ...logisticProfileBlockers,
    ...processProfileBlockers,
    ...cbsemPointRouteBlockers,
    ...(unifiedSemPlan?.route !== "legacy" ? unifiedSemPlan?.blockers ?? [] : []),
    ...unifiedReadinessBlockers.map((blocker) => blocker.detail),
    ...(unifiedRouteSelected && !onUnifiedSemAction
      ? ["The unified calculation controller is unavailable. Close this setup and reopen the active project before calculating."]
      : []),
  ])];
  const unifiedPrimaryBlocker = unifiedRouteSelected
    ? unifiedSemPrimaryBlockingMessageV1(unifiedSemPlan)
    : null;
  const fallbackPrimaryBlocker = !unifiedRouteSelected && !readiness.canRun
    ? readiness.blockers[0]?.detail ?? readiness.summary
    : methodProfileBlockers[0];
  const primaryCalculationBlocker: NativeCalculationBlockingMessageV1 | null = unifiedPrimaryBlocker
    ?? (fallbackPrimaryBlocker ? { cause: fallbackPrimaryBlocker, correction: null } : null);
  const canStart = !registryUnavailableReason
    && selectedMethodVisible
    && (unifiedRouteSelected ? unifiedSemPlan?.canStart === true : readiness.canRun)
    && (kind !== "mga" || (groupProfileState.status === "ready" && groupProfileAssessment.canRun))
    && logisticProfileBlockers.length === 0
    && processProfileBlockers.length === 0
    && !archivedCbsemBootstrapSetting
    && unifiedReadinessBlockers.length === 0
    && (!unifiedRouteSelected || Boolean(onUnifiedSemAction))
    && (unifiedSemPlan?.route === "legacy" || unifiedSemPlan?.canStart !== false);
  const verifiedLogisticProfile = logisticSelected
    && currentLogisticProfileState.status === "ready"
    && logisticProfileAssessment?.canRun
    && !logisticProfileAssessment.profileRequired
    ? currentLogisticProfileState.profile
    : undefined;
  const verifiedProcessProfile = processSelected
    && currentProcessProfileState.status === "ready"
    && processProfileAssessment?.canRun
    && !processProfileAssessment.profileRequired
    ? currentProcessProfileState.profile
    : undefined;
  const retryProcessProfile = () => {
    setProcessProfileState(retryNativeProcessProfileState(processProfileKey));
    setProcessProfileRetryNonce((value) => value + 1);
  };

  useEffect(() => {
    setCbsemInference(unifiedSem?.config.inference.kind === "case_bootstrap" ? "case_bootstrap" : "point");
    setModeratedMediationSelection({ authorityKey: unifiedSem?.authorityKey ?? null, pathId: undefined });
  }, [unifiedSem?.authorityKey]);

  useEffect(() => {
    if (kind !== "mga" || !groupColumn) {
      setGroupProfileState({ status: "idle", profile: null, error: null });
      return;
    }

    let active = true;
    const resident = residentDatasetGroupProfile(dataset, groupColumn, stableAnalysisColumns);
    if (resident) {
      setGroupProfileState({ status: "ready", profile: resident, error: null });
      return;
    }
    if (!isNativeDesktop()) {
      setGroupProfileState({
        status: "failed",
        profile: null,
        error: "Full-dataset group profiling is available in the installed desktop app.",
      });
      return;
    }

    setGroupProfileState({ status: "loading", profile: null, error: null });
    void profileNativeDatasetGroups(dataset.id, groupColumn, stableAnalysisColumns)
      .then((profile) => {
        if (active) setGroupProfileState({ status: "ready", profile, error: null });
      })
      .catch((error: unknown) => {
        if (!active) return;
        setGroupProfileState({
          status: "failed",
          profile: null,
          error: error instanceof Error ? error.message : String(error),
        });
      });
    return () => { active = false; };
  }, [analysisColumnKey, dataset, groupColumn, kind]);

  useEffect(() => {
    if (!logisticSelected || residentLogisticProfile || !logisticProfileAssessment?.canRun) {
      return;
    }
    if (!isNativeDesktop()) {
      setLogisticProfileState({
        status: "failed",
        key: logisticProfileKey,
        profile: null,
        error: "Open the installed desktop app to profile every row for binary logistic regression.",
      });
      return;
    }

    let cancelled = false;
    setLogisticProfileState({ status: "loading", key: logisticProfileKey, profile: null, error: null });
    void profileNativeLogisticDataset(dataset, settings, undefined, () => cancelled)
      .then((profile) => {
        if (!cancelled) setLogisticProfileState({ status: "ready", key: logisticProfileKey, profile, error: null });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setLogisticProfileState({
          status: "failed",
          key: logisticProfileKey,
          profile: null,
          error: error instanceof Error ? error.message : String(error),
        });
      });
    return () => { cancelled = true; };
  }, [dataset, logisticProfileAssessment?.canRun, logisticProfileKey, logisticSelected, residentLogisticProfile, settings]);

  useEffect(() => {
    if (!shouldStartNativeProcessProfile(
      processSelected,
      residentProcessProfile,
      processProfileAssessment?.canRun,
      processProfileKey,
      processProfileState,
    )) return;
    if (!isNativeDesktop()) {
      setProcessProfileState({
        status: "failed",
        key: processProfileKey,
        profile: null,
        error: "Open the installed desktop app to profile every row for graph-defined path analysis.",
      });
      return;
    }
    let cancelled = false;
    setProcessProfileState({ status: "loading", key: processProfileKey, profile: null, error: null });
    void profileNativeProcessDataset(dataset, processProfileSettings, undefined, () => cancelled)
      .then((profile) => {
        if (!cancelled) setProcessProfileState({ status: "ready", key: processProfileKey, profile, error: null });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setProcessProfileState({
          status: "failed",
          key: processProfileKey,
          profile: null,
          error: error instanceof Error ? error.message : String(error),
        });
      });
    return () => { cancelled = true; };
  }, [dataset, processProfileAssessment?.canRun, processProfileKey, processProfileRetryNonce, processProfileSettings, processSelected, residentProcessProfile]);

  useEffect(() => {
    if (kind !== "mga" || groupProfileState.status !== "ready") return;
    const values = groupProfileState.profile.groups.map((group) => group.value);
    const currentA = settings.groupAValue?.trim() ?? "";
    const currentB = settings.groupBValue?.trim() ?? "";
    const groupAValue = values.includes(currentA) ? currentA : values[0] ?? null;
    const groupBValue = values.includes(currentB) && currentB !== groupAValue
      ? currentB
      : values.find((value) => value !== groupAValue) ?? null;
    if (groupAValue !== (settings.groupAValue ?? null) || groupBValue !== (settings.groupBValue ?? null)) {
      setSettings({ groupAValue, groupBValue });
    }
  }, [groupProfileState, kind, setSettings, settings.groupAValue, settings.groupBValue]);

  useEffect(() => {
    if (!selectedMethodVisible || kind !== "ipma") return;
    const currentTarget = settings.ipmaTargets?.trim() ?? "";
    const selectedTarget = ipmaTargetOptions.some((option) => option.id === currentTarget)
      ? currentTarget
      : ipmaTargetOptions.length === 1
        ? ipmaTargetOptions[0].id
        : null;
    if (selectedTarget !== (settings.ipmaTargets ?? null)) setSettings({ ipmaTargets: selectedTarget });
  }, [ipmaTargetOptions, kind, selectedMethodVisible, setSettings, settings.ipmaTargets]);

  useEffect(() => {
    const nextFocused = filteredMethods.some((method) => method.kind === kind)
      ? kind
      : filteredMethods[0]?.kind;
    if (nextFocused) setFocusedKind(nextFocused);
  }, [filteredMethods, kind]);

  useEffect(() => {
    if (!filteredMethods.some((method) => method.kind === kind)) return;
    scrollNativeMethodOptionIntoView(optionRefs.current[kind]);
  }, [filteredMethods, kind]);

  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if (running || !(event.ctrlKey || event.metaKey) || event.key.toLocaleLowerCase() !== "f") return;
      event.preventDefault();
      searchRef.current?.focus();
      searchRef.current?.select();
    };
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, [running]);

  const focusMethod = (nextKind: NativeWorkbenchAnalysisKind) => {
    setFocusedKind(nextKind);
    optionRefs.current[nextKind]?.focus();
  };

  const selectMethod = (nextKind: NativeWorkbenchAnalysisKind) => {
    setFocusedKind(nextKind);
    setKind(nextKind);
  };

  const moveMethodFocus = (
    event: ReactKeyboardEvent<HTMLButtonElement>,
    currentKind: NativeWorkbenchAnalysisKind,
  ) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End", "Enter"].includes(event.key)) return;
    event.preventDefault();

    if (event.key === "Enter") {
      selectMethod(currentKind);
      return;
    }
    if (filteredMethods.length === 0) return;

    const currentIndex = Math.max(0, filteredMethods.findIndex((method) => method.kind === currentKind));
    const nextIndex = event.key === "Home"
      ? 0
      : event.key === "End"
        ? filteredMethods.length - 1
        : (currentIndex + (event.key === "ArrowDown" ? 1 : -1) + filteredMethods.length) % filteredMethods.length;
    focusMethod(filteredMethods[nextIndex].kind);
  };

  const focusFirstSearchResult = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "Enter") return;
    event.preventDefault();
    const first = filteredMethods.find((method) => method.kind === focusedKind) ?? filteredMethods[0];
    if (first) focusMethod(first.kind);
  };

  return (
    <form
      className="nd-calculation-dialog"
      onSubmit={(event) => {
        event.preventDefault();
        if (running || !canStart) return;
        dispatchNativeCalculationStartV1(
          unifiedSemPlan,
          onUnifiedSemAction,
          start,
          verifiedLogisticProfile ?? verifiedProcessProfile,
        );
      }}
    >
      <aside className="nd-dialog-sidebar" aria-label="Analysis methods">
        <label className="nd-method-search" htmlFor="nd-calculation-method-search">
          <span>Find a method</span>
          <span className="nd-search-input">
            <Search size={13} aria-hidden="true" />
            <input
              id="nd-calculation-method-search"
              ref={searchRef}
              autoFocus
              type="search"
              value={query}
              disabled={running}
              placeholder="Search methods"
              aria-controls="nd-calculation-method-list"
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={focusFirstSearchResult}
            />
          </span>
        </label>
        <span className="nd-method-count" role="status" aria-live="polite" aria-atomic="true">
          {filteredMethods.length === 1 ? "1 method" : `${filteredMethods.length} methods`}
        </span>
        <div
          id="nd-calculation-method-list"
          className="nd-method-list"
          role="listbox"
          aria-label="Available calculation methods"
        >
          {CATEGORY_ORDER.map((categoryId) => {
            const methods = filteredMethods.filter((method) => method.categoryId === categoryId);
            if (methods.length === 0) return null;
            const categoryLabelId = `nd-calculation-category-${categoryId}`;
            return (
              <div key={categoryId} role="group" aria-labelledby={categoryLabelId}>
                <div id={categoryLabelId} className="nd-method-category">{methods[0].categoryLabel}</div>
                {methods.map((method) => {
                  const Icon = METHOD_ICONS[method.kind];
                  const selected = kind === method.kind;
                  return (
                    <button
                      key={method.kind}
                      ref={(element) => { optionRefs.current[method.kind] = element; }}
                      id={optionId(method.kind)}
                      type="button"
                      role="option"
                      aria-selected={selected}
                      tabIndex={rovingKind === method.kind ? 0 : -1}
                      className={selected ? "active" : ""}
                      disabled={running}
                      onFocus={() => setFocusedKind(method.kind)}
                      onClick={() => selectMethod(method.kind)}
                      onKeyDown={(event) => moveMethodFocus(event, method.kind)}
                    >
                      <Icon size={15} aria-hidden="true" />
                      <span>
                        <strong>{method.label}</strong>
                      </span>
                    </button>
                  );
                })}
              </div>
            );
          })}
          {filteredMethods.length === 0 ? (
            <p className="nd-method-empty" role="status">
              {query.trim()
                ? `No available methods match "${query.trim()}".`
                : experimentalLabsEnabled
                  ? "No calculation methods are available with the current options."
                  : "No Standard methods are available yet. Enable Experimental Labs in Preferences to show executable experimental methods."}
            </p>
          ) : null}
        </div>
      </aside>

      <div className="nd-dialog-content">
        {running ? (
          <RunProgress methodLabel={selectedMethod.label} monitor={runMonitor} active />
        ) : selectedMethodVisible ? (
          <>
            <section
              id="nd-calculation-panel"
              className="nd-method-settings-panel"
              role="region"
              aria-labelledby={panelTitleId(kind)}
            >
              <header className="nd-method-settings-header">
                <div className="nd-method-settings-title-row">
                  <h3 id={panelTitleId(kind)}>{selectedMethod.label}</h3>
                  {openMethodDetails ? <button type="button" className="nd-method-details-link" onClick={openMethodDetails}>Method Details</button> : null}
                </div>
              </header>
              {unifiedSemPlan && unifiedSemPlan.route !== "legacy" ? <UnifiedSemFeatureSummary
                plan={unifiedSemPlan}
                context={unifiedSem}
                cbsemInference={cbsemInference}
                setCbsemInference={(inference) => {
                  setCbsemInference(inference);
                  if (kind !== "cbsem") return;
                  setSettings(inference === "case_bootstrap"
                    ? {
                        cbsemBootstrapSamples: Math.min(10_000, Math.max(500, settings.cbsemBootstrapSamples || 500)),
                        cbsemBootstrapInterval: "percentile_type7",
                        cbsemBootstrapTestTail: "two_sided",
                        confidenceLevel: 0.95,
                      }
                    : {
                        cbsemBootstrapSamples: 0,
                        cbsemBootstrapInterval: "percentile_type7",
                        cbsemBootstrapTestTail: "two_sided",
                      });
                }}
                onAction={onUnifiedSemAction}
                bootstrapOptions={unifiedBootstrapOptions}
                onModeratedMediationPathChange={(pathId) => setModeratedMediationSelection({
                  authorityKey: unifiedSem?.authorityKey ?? null,
                  pathId,
                })}
              /> : null}
              <MethodSettings
                kind={kind}
                settings={settings}
                setSettings={setSettings}
                dataset={dataset}
                analysisColumns={stableAnalysisColumns}
                nodes={nodes}
                edges={edges}
                unifiedSemPlan={unifiedSemPlan}
                groupProfileState={groupProfileState}
                groupProfileAssessment={groupProfileAssessment}
            logisticProfileState={currentLogisticProfileState}
            logisticProfileAssessment={logisticProfileAssessment}
            processProfileState={currentProcessProfileState}
            processProfileAssessment={processProfileAssessment}
            retryProcessProfile={retryProcessProfile}
              />
            </section>

            {primaryCalculationBlocker ? (
              <div className="nd-blocker" role="alert">
                <strong>Cannot start this calculation</strong>
                <span><strong>Cause:</strong> {primaryCalculationBlocker.cause}</span>
                {primaryCalculationBlocker.correction ? <span><strong>Correction:</strong> {primaryCalculationBlocker.correction}</span> : null}
              </div>
            ) : null}

            {runMonitor.status !== "idle" ? (
              <RunProgress methodLabel={selectedMethod.label} monitor={runMonitor} />
            ) : null}
          </>
        ) : (
          <section
            className="nd-method-settings-panel"
            role="status"
            aria-live="polite"
            aria-labelledby="nd-calculation-unavailable-title"
          >
            <header className="nd-method-settings-header">
              <h3 id="nd-calculation-unavailable-title">{registryUnavailableReason ? "Calculation catalogue unavailable" : "Calculation method unavailable"}</h3>
              <p>
                {registryUnavailableReason ?? (selectedCatalogEntry?.availability.internal_reason === "experimental_labs_disabled"
                  ? "Enable Experimental Labs in Preferences, then choose an available method."
                  : "The selected method or one of its optional add-ons is not currently available.")}
              </p>
            </header>
            <p className="nd-method-empty">
              Choose a method from the available list. Optional add-ons are treated as part of the method and can make a combined setup unavailable.
            </p>
          </section>
        )}
      </div>

      <footer>
        {running ? (
          <button type="button" onClick={cancel} disabled={runMonitor.status === "cancelling"}>
            {runMonitor.status === "cancelling" ? "Cancelling..." : "Cancel calculation"}
          </button>
        ) : (
          <>
            <button type="button" onClick={close}>Close</button>
            <button className="primary" type="submit" disabled={!canStart}>
              <Play size={14} aria-hidden="true" />
              {selectedMethodVisible
                ? nativeAnalysisStartLabel(kind, retry, settings.regressionType, settings.regressionBootstrap)
                : "Start calculation"}
            </button>
          </>
        )}
      </footer>
    </form>
  );
}

function UnifiedSemFeatureSummary({
  plan,
  context,
  cbsemInference,
  setCbsemInference,
  onAction,
  bootstrapOptions,
  onModeratedMediationPathChange,
}: {
  plan: UnifiedSemCalculationPlanV1;
  context: UnifiedSemCalculationContextV1 | null;
  cbsemInference: UnifiedSemInferenceChoiceV1;
  setCbsemInference: (inference: UnifiedSemInferenceChoiceV1) => void;
  onAction?: (action: UnifiedSemCalculationActionV1) => void;
  bootstrapOptions: { resamples: number; seed: number; confidenceLevel: number };
  onModeratedMediationPathChange: (pathId: string | null) => void;
}) {
  if (!context) return null;
  const higherOrderConstructs = plan.inventory?.higherOrderConstructs ?? [];
  const interactions = plan.inventory?.interactions ?? [];
  const indirectPathCount = plan.inventory?.indirectPathCount ?? 0;
  const indirectPathCountCapped = plan.inventory?.indirectPathCountCapped ?? false;
  let moderatedMediationCandidates: ReturnType<typeof unifiedSemModeratedMediationCandidatesV1> = [];
  try {
    moderatedMediationCandidates = unifiedSemModeratedMediationCandidatesV1(context, bootstrapOptions);
  } catch {
    moderatedMediationCandidates = [];
  }
  return <div className="nd-settings-grid" data-unified-sem-calculation={plan.route}>
    {plan.method !== "cbsem" && indirectPathCount > 0 ? <div className="nd-setting-note wide" id="nd-calculation-indirect-paths" role="status" aria-live="polite">
      <span>Indirect paths</span>
      <strong>{indirectPathCount}{indirectPathCountCapped ? "+" : ""} detected</strong>
    </div> : null}
    {interactions.map((interaction, index) => <div
      className="nd-setting-note wide"
      id={`nd-calculation-${interaction.order}-moderation-${index}`}
      data-interaction-term-id={interaction.termId}
      key={interaction.termId}
    >
      <span>{interaction.order === "three_way" ? "Three-way moderation" : "Moderation"}</span>
      <strong>{unifiedInteractionSummaryV1(interaction)}</strong>
    </div>)}
    {higherOrderConstructs.length > 0 ? <div className="nd-setting-note wide" id="nd-calculation-higher-order">
      <span>{higherOrderConstructs.length === 1 ? "Higher-order construct" : "Higher-order constructs"}</span>
      <span>
        <strong>{higherOrderConstructs.length === 1
          ? unifiedHigherOrderSummaryV1(higherOrderConstructs[0]!)
          : `${higherOrderConstructs.length} configured`}</strong>{" "}
        {higherOrderConstructs.length === 1 ? <button
          type="button"
          disabled={!onAction}
          aria-label={`Edit higher-order construct ${higherOrderConstructs[0]!.label}`}
          onClick={() => onAction?.({
            kind: "edit_higher_order",
            authorityKey: context.authorityKey,
            plan,
            higherOrderTermId: higherOrderConstructs[0]!.termId,
            higherOrderConstructId: higherOrderConstructs[0]!.constructId,
          })}
        >Edit…</button> : null}
      </span>
    </div> : null}
    {plan.method === "cbsem" ? <>
      <label className="wide" htmlFor="nd-calculation-cbsem-inference">Inference
        <select
          id="nd-calculation-cbsem-inference"
          value={cbsemInference}
          onChange={(event) => setCbsemInference(event.target.value as UnifiedSemInferenceChoiceV1)}
        >
          <option value="point">Maximum-likelihood point estimates</option>
          <option value="case_bootstrap">Case-resampling bootstrap</option>
        </select>
      </label>
      <button
        type="button"
        className="wide"
        disabled={!onAction}
        onClick={() => onAction?.({
          kind: "open_advanced_parameter_table",
          authorityKey: context.authorityKey,
          plan,
        })}
      >Advanced Parameter Table</button>
    </> : null}
    {plan.method === "pls_bootstrap" && plan.moderatedMediation?.candidateCount ? <div
      className="nd-setting-note wide"
      id="nd-calculation-moderated-mediation"
    >
      <span>Moderated mediation</span>
      <strong>{plan.moderatedMediation.selectedPath
        ? `${plan.moderatedMediation.selectedPath.xLabel} → ${plan.moderatedMediation.selectedPath.mediatorLabel} → ${plan.moderatedMediation.selectedPath.yLabel}; ${plan.moderatedMediation.fixedTargetSummary}.`
        : `${plan.moderatedMediation.candidateCount} eligible two-relation paths; choose one to add the fixed five-target inference.`}</strong>
      <label htmlFor="nd-calculation-moderated-mediation-path">Indirect path
        <select
          id="nd-calculation-moderated-mediation-path"
          value={plan.moderatedMediation.selectedPath?.pathId ?? ""}
          onChange={(event) => onModeratedMediationPathChange(event.target.value || null)}
        >
          <option value="">Do not add moderated-mediation inference</option>
          {moderatedMediationCandidates.map((candidate) => <option key={candidate.pathId} value={candidate.pathId}>
            {candidate.xLabel} → {candidate.mediatorLabel} → {candidate.yLabel} ({candidate.moderatedStage === "first_stage" ? "first stage" : "second stage"})
          </option>)}
        </select>
      </label>
    </div> : null}
  </div>;
}

function MethodSettings({
  kind,
  settings,
  setSettings,
  dataset,
  analysisColumns,
  groupProfileState,
  groupProfileAssessment,
  logisticProfileState,
  logisticProfileAssessment,
  processProfileState,
  processProfileAssessment,
  retryProcessProfile,
  nodes,
  edges,
  unifiedSemPlan,
}: Pick<NativeCalculationDialogProps, "kind" | "settings" | "setSettings" | "dataset" | "analysisColumns" | "nodes" | "edges"> & {
  groupProfileState: GroupProfileState;
  groupProfileAssessment: NativeMgaProfileAssessment;
  logisticProfileState: LogisticProfileState;
  logisticProfileAssessment: NativeLogisticReadinessAssessment | null;
  processProfileState: ProcessProfileState;
  processProfileAssessment: NativeProcessReadinessAssessment | null;
  retryProcessProfile: () => void;
  unifiedSemPlan: UnifiedSemCalculationPlanV1 | null;
}) {
  const numericColumns = useMemo(() => nativeNumericCaseWeightColumns(dataset), [dataset]);
  const ncaNumericColumns = useMemo(() => nativeNcaNumericColumns(dataset), [dataset]);
  const pcaNumericColumns = useMemo(() => nativePcaNumericColumns(dataset), [dataset]);
  const olsNumericColumns = useMemo(() => nativeOlsNumericColumns(dataset), [dataset]);
  const groupColumns = useMemo(
    () => nativeEligibleGroupColumns(dataset, analysisColumns),
    [analysisColumns, dataset],
  );
  const caseWeightColumn = settings.caseWeightColumn?.trim() ?? "";
  const selectedWeightIsEligible = !caseWeightColumn || numericColumns.includes(caseWeightColumn);
  const regressionBootstrap = kind === "regression" && settings.regressionBootstrap === true;
  const unifiedPls = (kind === "pls_algorithm" || kind === "pls_bootstrap")
    && unifiedSemPlan?.route === "general_sem_pls";
  const unifiedCbsem = kind === "cbsem"
    && (unifiedSemPlan?.route === "general_sem_cbsem"
      || unifiedSemPlan?.route === "exact_cbsem_compatibility");
  const cbsemBootstrap = kind === "cbsem" && (unifiedCbsem
    ? unifiedSemPlan.inference === "case_bootstrap"
    : (settings.cbsemBootstrapSamples ?? 0) > 0);
  const cbsemAnalyticStudentized = cbsemBootstrap
    && (settings.cbsemBootstrapInterval ?? "percentile_type7") === "analytic_studentized_type7";
  const cbsemBcaType7 = cbsemBootstrap
    && (settings.cbsemBootstrapInterval ?? "percentile_type7") === "bca_type7";
  const cbsemBoundedLabsInterval = cbsemAnalyticStudentized || cbsemBcaType7;
  const resampling = kind === "pls_bootstrap" || kind === "plsc_bootstrap" || kind === "pls_permutation" || kind === "pls_posthoc_technical_minimum_sample_size" || kind === "pls_sample_size_power" || kind === "mga" || kind === "predict" || kind === "nca" || regressionBootstrap || cbsemBootstrap;
  const selectedGroupColumn = settings.groupColumn?.trim() ?? "";
  const selectedGroupColumnEligible = !selectedGroupColumn || groupColumns.includes(selectedGroupColumn);
  const groupValues = groupProfileState.profile?.groups ?? [];
  const ipmaTargets = useMemo(() => nativeIpmaTargetOptions(nodes, edges), [edges, nodes]);
  const selectedIpmaTarget = settings.ipmaTargets?.trim() ?? "";
  const selectedNcaX = settings.ncaX?.trim() ?? "";
  const selectedNcaY = settings.ncaY?.trim() ?? "";
  const selectedPcaVariables = nativePcaSelectedVariables(settings);
  const selectedPcaVariableSet = new Set(selectedPcaVariables);
  const selectedOlsOutcome = settings.regressionOutcome?.trim() ?? "";
  const selectedOlsPredictors = nativeOlsCsvValues(settings.regressionPredictors);
  const selectedOlsControls = nativeOlsCsvValues(settings.regressionControls);
  const selectedOlsPredictorSet = new Set(selectedOlsPredictors);
  const selectedOlsControlSet = new Set(selectedOlsControls);
  const ctaPlsBlocks = useMemo(() => nativeCtaPlsEligibleBlocks(nodes), [nodes]);
  const powerEligibleConstructs = useMemo(() => nodes.filter((node) => (
    !node.data.semantic
    && node.data.mode === "reflective"
    && node.data.indicators.length >= 3
    && node.data.indicators.length <= 10
  )), [nodes]);
  const selectedPowerPredictor = settings.plsPowerPredictorConstruct?.trim() ?? "";
  const selectedPowerOutcome = settings.plsPowerOutcomeConstruct?.trim() ?? "";
  const selectedPowerPredictorNode = powerEligibleConstructs.find((node) => node.id === selectedPowerPredictor);
  const selectedPowerOutcomeNode = powerEligibleConstructs.find((node) => node.id === selectedPowerOutcome);
  const powerWorkload = useMemo(() => {
    if (kind !== "pls_sample_size_power") return null;
    try {
      return buildNativePlsSampleSizePowerRecipe({
        scenarioIdentity: settings.plsPowerScenarioIdentity ?? "",
        predictorConstruct: selectedPowerPredictor,
        outcomeConstruct: selectedPowerOutcome,
        predictorIndicatorLoadings: settings.plsPowerPredictorLoadings ?? "",
        outcomeIndicatorLoadings: settings.plsPowerOutcomeLoadings ?? "",
        populationPath: String(settings.plsPowerPopulationPath ?? ""),
        exogenousDistribution: "standard_normal",
        structuralDisturbanceDistribution: "standard_normal",
        indicatorErrorDistribution: "standard_normal",
        missingData: "none",
        weightingScheme: settings.weightingScheme === "path" ? "path" : "",
        preprocessing: settings.preprocessing === "standardized" ? "standardized" : "",
        tolerance: String(settings.tolerance ?? ""),
        maxIterations: String(settings.maxIterations ?? ""),
        inference: "case_bootstrap_null_centered_two_sided_plus_one",
        sampleSizeGrid: settings.plsPowerSampleSizeGrid ?? "",
        alpha: String(settings.plsPowerAlpha ?? ""),
        targetPower: String(settings.plsPowerTargetPower ?? ""),
        confidenceLevel: String(settings.confidenceLevel),
        monteCarloReplicates: String(settings.plsPowerMonteCarloReplicates ?? ""),
        bootstrapReplicates: String(settings.plsPowerBootstrapReplicates ?? ""),
        masterSeed: String(settings.seed),
        workers: String(settings.workers),
      }).workload;
    } catch {
      return null;
    }
  }, [kind, selectedPowerOutcome, selectedPowerPredictor, settings]);
  const logisticRegression = settings.regressionType === "logistic";
  const processRegression = settings.regressionType === "process";
  const regressionTermLimit = regressionBootstrap
    ? NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS
    : logisticRegression
      ? NATIVE_LOGISTIC_MAX_TERMS
      : NATIVE_OLS_MAX_TERMS;
  const initializedPcaSelection = useRef(false);
  const initializedOlsSelection = useRef(false);
  const initializedPowerSelection = useRef(false);
  useEffect(() => {
    if (kind !== "pls_sample_size_power" || initializedPowerSelection.current) return;
    const ordinaryPaths = edges.filter((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role !== "control" && role !== "covariance";
    });
    const path = ordinaryPaths.length === 1 ? ordinaryPaths[0] : null;
    const predictorNode = path ? powerEligibleConstructs.find((node) => node.id === path.source) : undefined;
    const outcomeNode = path ? powerEligibleConstructs.find((node) => node.id === path.target) : undefined;
    if (predictorNode && outcomeNode) {
      initializedPowerSelection.current = true;
      setSettings({
        plsPowerPredictorConstruct: predictorNode.id,
        plsPowerOutcomeConstruct: outcomeNode.id,
        plsPowerPredictorLoadings: settings.plsPowerPredictorLoadings
          ?? predictorNode.data.indicators.map(() => "0.80").join(","),
        plsPowerOutcomeLoadings: settings.plsPowerOutcomeLoadings
          ?? outcomeNode.data.indicators.map(() => "0.80").join(","),
      });
    }
  }, [edges, kind, powerEligibleConstructs, setSettings, settings.plsPowerOutcomeLoadings, settings.plsPowerPredictorLoadings]);
  useEffect(() => {
    if (kind !== "pca" || initializedPcaSelection.current) return;
    initializedPcaSelection.current = true;
    if (!selectedPcaVariables.length && pcaNumericColumns.length >= 2) {
      setSettings({ pcaVariables: pcaNumericColumns.slice(0, NATIVE_PCA_MAX_VARIABLES).join(",") });
    }
  }, [kind, pcaNumericColumns, selectedPcaVariables.length, setSettings]);
  useEffect(() => {
    if (kind !== "regression" || processRegression || initializedOlsSelection.current) return;
    initializedOlsSelection.current = true;
    if (!selectedOlsOutcome && !selectedOlsPredictors.length && olsNumericColumns.length >= 2) {
      setSettings({
        regressionType: logisticRegression ? "logistic" : "ols",
        regressionOutcome: olsNumericColumns[0],
        regressionPredictors: olsNumericColumns[1],
        regressionControls: null,
        robustSe: logisticRegression ? "none" : "hc3",
      });
    }
  }, [kind, logisticRegression, olsNumericColumns, processRegression, selectedOlsOutcome, selectedOlsPredictors.length, setSettings]);

  const setPcaVariableSelected = (variable: string, selected: boolean) => {
    const next = selected
      ? [...selectedPcaVariables, variable]
      : selectedPcaVariables.filter((candidate) => candidate !== variable);
    setSettings({ pcaVariables: next.join(",") || null });
  };
  const setOlsRoleSelected = (role: "predictor" | "control", variable: string, selected: boolean) => {
    const current = role === "predictor" ? selectedOlsPredictors : selectedOlsControls;
    const next = selected ? [...current, variable] : current.filter((candidate) => candidate !== variable);
    setSettings(role === "predictor"
      ? { regressionPredictors: next.join(",") || null }
      : { regressionControls: next.join(",") || null });
  };

  return (
    <fieldset>
      <legend>Method settings</legend>
      <div className="nd-settings-grid">
        {!unifiedPls && kind !== "pls_sample_size_power" && kind !== "nca" && kind !== "pca" && kind !== "regression" && kind !== "ipma" && kind !== "mga" && kind !== "cbsem" && kind !== "gsca" ? <label htmlFor="nd-calculation-weighting">Weighting scheme
          <select
            id="nd-calculation-weighting"
            value={settings.weightingScheme ?? "path"}
            onChange={(event) => setSettings({
              weightingScheme: event.target.value as NonNullable<AnalysisUiSettings["weightingScheme"]>,
            })}
          >
            <option value="path">Path weighting</option>
            <option value="factor">Factor weighting</option>
            <option value="pca" disabled={kind === "plsc" || kind === "plsc_bootstrap" || kind === "wpls" || kind === "cca" || kind === "cta_pls"}>PCA weighting</option>
          </select>
        </label> : null}

        {!unifiedPls && kind !== "pls_sample_size_power" && kind !== "nca" && kind !== "regression" && kind !== "pca" && kind !== "wpls" && kind !== "cca" && kind !== "ipma" && kind !== "mga" && kind !== "cbsem" && kind !== "gsca" ? (
          <label htmlFor="nd-calculation-preprocessing">Result data
            <select
              id="nd-calculation-preprocessing"
              value={settings.preprocessing ?? "standardized"}
              onChange={(event) => setSettings({
                preprocessing: event.target.value as NonNullable<AnalysisUiSettings["preprocessing"]>,
              })}
            >
              <option value="standardized">Standardized</option>
              <option value="mean_centered">Mean-centered</option>
              <option value="unstandardized">Unstandardized</option>
            </select>
          </label>
        ) : null}

        {kind !== "nca" && kind !== "pca" && kind !== "regression" && kind !== "gsca" ? <label htmlFor="nd-calculation-max-iterations">Maximum iterations
          <input
            id="nd-calculation-max-iterations"
            type="number"
            min={100}
            max={100_000}
            step={100}
            value={settings.maxIterations ?? 3_000}
            onChange={(event) => setSettings({ maxIterations: Number(event.target.value) })}
          />
        </label> : null}

        {kind !== "nca" && kind !== "pca" && kind !== "regression" && kind !== "gsca" ? <label htmlFor="nd-calculation-tolerance">Stop criterion
          <input
            id="nd-calculation-tolerance"
            type="number"
            min={1e-12}
            max={0.01}
            step="any"
            value={settings.tolerance ?? 1e-7}
            onChange={(event) => setSettings({ tolerance: Number(event.target.value) })}
          />
        </label> : null}

        {kind === "pls_bootstrap" || kind === "pls_posthoc_technical_minimum_sample_size" ? (
          <>
            <label htmlFor="nd-calculation-bootstrap-samples">Bootstrap samples
              <input
                id="nd-calculation-bootstrap-samples"
                type="number"
                min={NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap.min}
                max={NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap.max}
                step={NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap.step}
                value={Math.max(
                  NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap.min,
                  settings.bootstrapSamples || NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.default,
                )}
                onChange={(event) => setSettings({ bootstrapSamples: Number(event.target.value) })}
              />
            </label>
            <label htmlFor="nd-calculation-confidence">Confidence level
              <input
                id="nd-calculation-confidence"
                type="number"
                min={80}
                max={99.9}
                step={0.1}
                value={Number((settings.confidenceLevel * 100).toFixed(1))}
                onChange={(event) => setSettings({ confidenceLevel: Number(event.target.value) / 100 })}
              />
            </label>
            {kind === "pls_bootstrap" && !unifiedPls ? (
              <label htmlFor="nd-calculation-studentized">Studentized inner samples
                <select
                  id="nd-calculation-studentized"
                  value={settings.studentizedInnerSamples}
                  onChange={(event) => setSettings({ studentizedInnerSamples: Number(event.target.value) })}
                >
                  <option value={0}>Off</option>
                  {[99, 199, 299, 399, 499, 599, 699, 799, 899, 999].map((samples) => (
                    <option key={samples} value={samples}>{samples}</option>
                  ))}
                </select>
              </label>
            ) : null}
          </>
        ) : null}

        {kind === "plsc_bootstrap" ? (
          <>
            <label htmlFor="nd-calculation-plsc-bootstrap-samples">Bootstrap samples
              <input
                id="nd-calculation-plsc-bootstrap-samples"
                type="number"
                min={1_000}
                max={10_000}
                step={1_000}
                value={Math.min(10_000, Math.max(1_000, settings.bootstrapSamples || 10_000))}
                onChange={(event) => setSettings({ bootstrapSamples: Number(event.target.value) })}
              />
            </label>
            <label htmlFor="nd-calculation-plsc-bootstrap-confidence">Confidence level
              <input
                id="nd-calculation-plsc-bootstrap-confidence"
                type="number"
                min={80}
                max={99.9}
                step={0.1}
                value={Number((settings.confidenceLevel * 100).toFixed(1))}
                onChange={(event) => setSettings({ confidenceLevel: Number(event.target.value) / 100 })}
              />
            </label>
          </>
        ) : null}

        {kind === "pls_permutation" ? (
          <>
            <label htmlFor="nd-calculation-permutations">Permutations
              <input
                id="nd-calculation-permutations"
                type="number"
                min={NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.permutation.min}
                max={NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.permutation.max}
                step={NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.permutation.step}
                value={settings.permutationSamples}
                onChange={(event) => setSettings({ permutationSamples: Number(event.target.value) })}
              />
            </label>
          </>
        ) : null}

        {kind === "pls_sample_size_power" ? (
          <div className="nd-pca-settings wide" id="nd-calculation-pls-power-setup">
            <label htmlFor="nd-calculation-pls-power-scenario">Scenario identity
              <input
                id="nd-calculation-pls-power-scenario"
                required
                maxLength={80}
                value={settings.plsPowerScenarioIdentity ?? ""}
                onChange={(event) => setSettings({ plsPowerScenarioIdentity: event.target.value })}
              />
            </label>
            <label htmlFor="nd-calculation-pls-power-predictor">Predictor construct
              <select
                id="nd-calculation-pls-power-predictor"
                required
                value={selectedPowerPredictor}
                onChange={(event) => {
                  const next = powerEligibleConstructs.find((node) => node.id === event.target.value);
                  setSettings({
                    plsPowerPredictorConstruct: next?.id ?? null,
                    plsPowerPredictorLoadings: next?.data.indicators.map(() => "0.80").join(",") ?? null,
                  });
                }}
              >
                <option value="">Select the model predictor</option>
                {powerEligibleConstructs.map((node) => (
                  <option key={node.id} value={node.id} disabled={node.id === selectedPowerOutcome}>
                    {node.data.label} ({node.data.indicators.length} indicators)
                  </option>
                ))}
              </select>
            </label>
            <label htmlFor="nd-calculation-pls-power-outcome">Outcome construct
              <select
                id="nd-calculation-pls-power-outcome"
                required
                value={selectedPowerOutcome}
                onChange={(event) => {
                  const next = powerEligibleConstructs.find((node) => node.id === event.target.value);
                  setSettings({
                    plsPowerOutcomeConstruct: next?.id ?? null,
                    plsPowerOutcomeLoadings: next?.data.indicators.map(() => "0.80").join(",") ?? null,
                  });
                }}
              >
                <option value="">Select the model outcome</option>
                {powerEligibleConstructs.map((node) => (
                  <option key={node.id} value={node.id} disabled={node.id === selectedPowerPredictor}>
                    {node.data.label} ({node.data.indicators.length} indicators)
                  </option>
                ))}
              </select>
            </label>
            <label htmlFor="nd-calculation-pls-power-predictor-loadings">Predictor loadings
              <input
                id="nd-calculation-pls-power-predictor-loadings"
                required
                value={settings.plsPowerPredictorLoadings ?? ""}
                aria-describedby="nd-calculation-pls-power-loading-help"
                onChange={(event) => setSettings({ plsPowerPredictorLoadings: event.target.value })}
              />
              <small>{selectedPowerPredictorNode?.data.indicators.join(", ") || "Choose a predictor"}</small>
            </label>
            <label htmlFor="nd-calculation-pls-power-outcome-loadings">Outcome loadings
              <input
                id="nd-calculation-pls-power-outcome-loadings"
                required
                value={settings.plsPowerOutcomeLoadings ?? ""}
                aria-describedby="nd-calculation-pls-power-loading-help"
                onChange={(event) => setSettings({ plsPowerOutcomeLoadings: event.target.value })}
              />
              <small>{selectedPowerOutcomeNode?.data.indicators.join(", ") || "Choose an outcome"}</small>
            </label>
            <div className="nd-setting-note wide" id="nd-calculation-pls-power-loading-help">
              <span>Loading contract</span>
              <strong>One finite loading from 0.50 to 0.95 per listed indicator, in displayed order.</strong>
            </div>
            <label htmlFor="nd-calculation-pls-power-path">Population path
              <input id="nd-calculation-pls-power-path" type="number" min={-0.8} max={0.8} step={0.01} value={settings.plsPowerPopulationPath ?? 0.30} onChange={(event) => setSettings({ plsPowerPopulationPath: Number(event.target.value) })} />
            </label>
            <label htmlFor="nd-calculation-pls-power-grid">Sample-size grid
              <input id="nd-calculation-pls-power-grid" required value={settings.plsPowerSampleSizeGrid ?? ""} onChange={(event) => setSettings({ plsPowerSampleSizeGrid: event.target.value })} />
              <small>2-16 strictly increasing values from 30 to 5,000; no interpolation or extrapolation.</small>
            </label>
            <label htmlFor="nd-calculation-pls-power-alpha">Two-sided alpha
              <input id="nd-calculation-pls-power-alpha" type="number" min={0.001} max={0.1} step={0.001} value={settings.plsPowerAlpha ?? 0.05} onChange={(event) => setSettings({ plsPowerAlpha: Number(event.target.value) })} />
            </label>
            <label htmlFor="nd-calculation-pls-power-target">Target power
              <input id="nd-calculation-pls-power-target" type="number" min={0.5} max={0.99} step={0.01} value={settings.plsPowerTargetPower ?? 0.80} onChange={(event) => setSettings({ plsPowerTargetPower: Number(event.target.value) })} />
            </label>
            <label htmlFor="nd-calculation-pls-power-confidence">Wilson confidence level
              <input id="nd-calculation-pls-power-confidence" type="number" min={80} max={99.9} step={0.1} value={Number((settings.confidenceLevel * 100).toFixed(1))} onChange={(event) => setSettings({ confidenceLevel: Number(event.target.value) / 100 })} />
            </label>
            <label htmlFor="nd-calculation-pls-power-mc">Monte Carlo replicates per grid point
              <input id="nd-calculation-pls-power-mc" type="number" min={100} max={10_000} step={1} value={settings.plsPowerMonteCarloReplicates ?? 250} onChange={(event) => setSettings({ plsPowerMonteCarloReplicates: Number(event.target.value) })} />
            </label>
            <label htmlFor="nd-calculation-pls-power-bootstrap">Case-bootstrap replicates per dataset
              <input id="nd-calculation-pls-power-bootstrap" type="number" min={99} max={1_999} step={2} value={settings.plsPowerBootstrapReplicates ?? 199} onChange={(event) => setSettings({ plsPowerBootstrapReplicates: Number(event.target.value) })} />
            </label>
            <label htmlFor="nd-calculation-pls-power-workers">Parallel workers
              <input id="nd-calculation-pls-power-workers" type="number" min={1} max={64} step={1} value={settings.workers} onChange={(event) => setSettings({ workers: Number(event.target.value) })} />
            </label>
            <div className="nd-setting-note wide" id="nd-calculation-pls-power-workload" role="status" aria-live="polite">
              <span>Pre-run desktop workload</span>
              <strong>{powerWorkload
                ? `${powerWorkload.plannedDatasets.toLocaleString("en-US")} independent datasets; ${powerWorkload.estimatedPlsFits.toLocaleString("en-US")} PLS fits; ${powerWorkload.estimatedPlsCaseFits.toLocaleString("en-US")} fitted rows. Hard caps: 250,000 fits and 100,000,000 fitted rows.`
                : "Complete a valid plan to calculate workload. Plans above 250,000 fits or 100,000,000 fitted rows are blocked."}</strong>
            </div>
          </div>
        ) : null}

        {kind === "mga" ? (
          <div className="nd-mga-settings wide" aria-busy={groupProfileState.status === "loading"}>
            <label htmlFor="nd-calculation-group-column">Grouping variable
              <select
                id="nd-calculation-group-column"
                value={selectedGroupColumn}
                onChange={(event) => setSettings({
                  groupColumn: event.target.value || null,
                  groupAValue: null,
                  groupBValue: null,
                })}
              >
                <option value="">Select an unassigned variable</option>
                {!selectedGroupColumnEligible ? (
                  <option value={selectedGroupColumn} disabled>{selectedGroupColumn} (used as an indicator)</option>
                ) : null}
                {groupColumns.map((column) => <option key={column} value={column}>{column}</option>)}
              </select>
            </label>

            {groupProfileState.status === "loading" ? <p role="status">Reading complete-dataset group counts...</p> : null}
            {groupProfileState.status === "ready" ? (
              <div className="nd-mga-group-grid">
                <label htmlFor="nd-calculation-group-a">Group A
                  <select
                    id="nd-calculation-group-a"
                    value={settings.groupAValue ?? ""}
                    onChange={(event) => setSettings({ groupAValue: event.target.value || null })}
                  >
                    <option value="">Select Group A</option>
                    {groupValues.map((group) => (
                      <option key={group.value} value={group.value} disabled={group.value === settings.groupBValue}>
                        {nativeGroupOptionLabel(group)}
                      </option>
                    ))}
                  </select>
                </label>
                <label htmlFor="nd-calculation-group-b">Group B
                  <select
                    id="nd-calculation-group-b"
                    value={settings.groupBValue ?? ""}
                    onChange={(event) => setSettings({ groupBValue: event.target.value || null })}
                  >
                    <option value="">Select Group B</option>
                    {groupValues.map((group) => (
                      <option key={group.value} value={group.value} disabled={group.value === settings.groupAValue}>
                        {nativeGroupOptionLabel(group)}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            ) : null}

            <label htmlFor="nd-calculation-group-permutations">Permutations
              <input
                id="nd-calculation-group-permutations"
                type="number"
                min={NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.minimum}
                max={NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.maximum}
                step={1}
                value={settings.groupPermutationSamples ?? NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.default}
                onChange={(event) => setSettings({ groupPermutationSamples: Number(event.target.value) })}
              />
            </label>
            <label htmlFor="nd-calculation-micom-confidence">Confidence level
              <input
                id="nd-calculation-micom-confidence"
                type="number"
                min={80}
                max={99.9}
                step={0.1}
                value={Number((settings.confidenceLevel * 100).toFixed(1))}
                onChange={(event) => setSettings({ confidenceLevel: Number(event.target.value) / 100 })}
              />
            </label>
            {groupProfileAssessment.warnings.length ? (
              <ul className="nd-setting-warnings" aria-label="Excluded group rows">
                {groupProfileAssessment.warnings.map((warning) => <li key={warning}>{warning}</li>)}
              </ul>
            ) : null}
            <label className="nd-micom-confirmation wide" htmlFor="nd-calculation-micom-configural">
              <input
                id="nd-calculation-micom-configural"
                type="checkbox"
                checked={settings.micomConfiguralConfirmed === true}
                onChange={(event) => setSettings({ micomConfiguralConfirmed: event.target.checked })}
              />
              <span><strong>Confirm MICOM Step 1 (configural invariance)</strong> Both groups use identical indicators, coding, data treatment, model and algorithm settings, and the constructs have the same substantive meaning.</span>
            </label>
          </div>
        ) : null}

        {kind === "ipma" ? (
          <label className="wide" htmlFor="nd-calculation-ipma-target">Endogenous target
            <select
              id="nd-calculation-ipma-target"
              required
              value={selectedIpmaTarget}
              onChange={(event) => setSettings({ ipmaTargets: event.target.value || null })}
            >
              <option value="">Select one endogenous construct</option>
              {ipmaTargets.map((target) => <option key={target.id} value={target.id}>{target.optionLabel}</option>)}
            </select>
          </label>
        ) : null}

        {kind === "cbsem" ? (
          <>
            {!unifiedCbsem ? <label className="wide" htmlFor="nd-calculation-cbsem-model-type">Model type
              <select
                id="nd-calculation-cbsem-model-type"
                value={settings.cbsemModelType ?? "sem"}
                onChange={(event) => setSettings({
                  cbsemModelType: event.target.value as NonNullable<AnalysisUiSettings["cbsemModelType"]>,
                  ...(event.target.value === "cfa" ? {} : {
                    cbsemBootstrapInterval: "percentile_type7" as const,
                    cbsemBootstrapTestTail: "two_sided" as const,
                  }),
                })}
              >
                <option value="sem">Structural equation model (paths required)</option>
                <option value="cfa">Confirmatory factor analysis (no paths)</option>
              </select>
            </label> : null}
            {!unifiedCbsem && cbsemBootstrap ? <div className="nd-inline-warning wide" id="nd-calculation-cbsem-archived-bootstrap" role="alert">
              <strong>Clear the archived bootstrap setting before running this point-estimate setup.</strong>
              <button type="button" onClick={() => setSettings({
                cbsemBootstrapSamples: 0,
                cbsemBootstrapInterval: "percentile_type7",
                cbsemBootstrapTestTail: "two_sided",
                workers: 1,
              })}>Clear setting</button>
            </div> : null}
            {unifiedCbsem && cbsemBootstrap ? <>
              <label htmlFor="nd-calculation-cbsem-bootstrap-samples">Bootstrap samples
                <input
                  id="nd-calculation-cbsem-bootstrap-samples"
                  type="number"
                  min={500}
                  max={10_000}
                  step={1}
                  value={Math.min(10_000, Math.max(500, settings.cbsemBootstrapSamples || 500))}
                  onChange={(event) => setSettings({ cbsemBootstrapSamples: Number(event.target.value) })}
                />
              </label>
              <div className="nd-setting-note wide" id="nd-calculation-cbsem-bootstrap-contract" role="note">
                <span>Inference contract</span>
                <strong>Two-sided 95% percentile intervals with complete model refitting.</strong>
              </div>
            </> : null}
          </>
        ) : null}

        {kind === "nca" ? (
          <>
            <label htmlFor="nd-calculation-nca-x">Condition variable (X)
              <select
                id="nd-calculation-nca-x"
                required
                value={selectedNcaX}
                onChange={(event) => setSettings({ ncaX: event.target.value || null })}
              >
                <option value="">Select a numeric variable</option>
                {ncaNumericColumns.map((column) => (
                  <option key={column} value={column} disabled={column === selectedNcaY}>{column}</option>
                ))}
              </select>
            </label>
            <label htmlFor="nd-calculation-nca-y">Outcome variable (Y)
              <select
                id="nd-calculation-nca-y"
                required
                value={selectedNcaY}
                onChange={(event) => setSettings({ ncaY: event.target.value || null })}
              >
                <option value="">Select a different numeric variable</option>
                {ncaNumericColumns.map((column) => (
                  <option key={column} value={column} disabled={column === selectedNcaX}>{column}</option>
                ))}
              </select>
            </label>
            <label htmlFor="nd-calculation-nca-ceiling">Ceiling line
              <select
                id="nd-calculation-nca-ceiling"
                value={settings.ncaCeiling ?? "both"}
                onChange={(event) => setSettings({
                  ncaCeiling: event.target.value as NonNullable<AnalysisUiSettings["ncaCeiling"]>,
                })}
              >
                <option value="both">CE-FDH and CR-FDH</option>
                <option value="ce_fdh">CE-FDH</option>
                <option value="cr_fdh">CR-FDH</option>
              </select>
            </label>
            <label htmlFor="nd-calculation-nca-permutations">Permutations
              <input
                id="nd-calculation-nca-permutations"
                type="number"
                min={NATIVE_NCA_MIN_PERMUTATIONS}
                max={NATIVE_NCA_MAX_PERMUTATIONS}
                step={1}
                value={settings.ncaPermutationSamples ?? NATIVE_NCA_DEFAULT_PERMUTATIONS}
                onChange={(event) => setSettings({ ncaPermutationSamples: Number(event.target.value) })}
              />
            </label>
          </>
        ) : null}

        {kind === "pca" ? (
          <div className="nd-pca-settings wide">
            <fieldset className="nd-pca-variables">
              <legend>Variables ({selectedPcaVariables.length} selected)</legend>
              <div className="nd-pca-variable-actions">
                <button
                  type="button"
                  onClick={() => setSettings({
                    pcaVariables: pcaNumericColumns.slice(0, NATIVE_PCA_MAX_VARIABLES).join(",") || null,
                  })}
                >Select all numeric</button>
                <button type="button" onClick={() => setSettings({ pcaVariables: null })}>Clear</button>
              </div>
              <div className="nd-pca-variable-list">
                {pcaNumericColumns.map((variable) => (
                  <label key={variable}>
                    <input
                      type="checkbox"
                      checked={selectedPcaVariableSet.has(variable)}
                      disabled={!selectedPcaVariableSet.has(variable) && selectedPcaVariables.length >= NATIVE_PCA_MAX_VARIABLES}
                      onChange={(event) => setPcaVariableSelected(variable, event.target.checked)}
                    />
                    <span>{variable}</span>
                  </label>
                ))}
              </div>
            </fieldset>
            <label htmlFor="nd-calculation-pca-rule">Component retention
              <select
                id="nd-calculation-pca-rule"
                value={settings.pcaComponentRule ?? "kaiser"}
                onChange={(event) => setSettings({
                  pcaComponentRule: event.target.value as NonNullable<AnalysisUiSettings["pcaComponentRule"]>,
                })}
              >
                <option value="kaiser">Kaiser criterion (eigenvalue at least 1)</option>
                <option value="fixed">Fixed component count</option>
                <option value="variance_threshold">Cumulative variance threshold</option>
              </select>
            </label>
            {settings.pcaComponentRule === "fixed" ? (
              <label htmlFor="nd-calculation-pca-components">Components
                <input
                  id="nd-calculation-pca-components"
                  type="number"
                  min={1}
                  max={Math.max(1, Math.min(selectedPcaVariables.length, NATIVE_PCA_MAX_VARIABLES))}
                  step={1}
                  value={settings.pcaComponents ?? 2}
                  onChange={(event) => setSettings({ pcaComponents: Number(event.target.value) })}
                />
              </label>
            ) : null}
            {settings.pcaComponentRule === "variance_threshold" ? (
              <label htmlFor="nd-calculation-pca-threshold">Cumulative variance target (%)
                <input
                  id="nd-calculation-pca-threshold"
                  type="number"
                  min={1}
                  max={99.9}
                  step={0.1}
                  value={Number(((settings.pcaVarianceThreshold ?? 0.80) * 100).toFixed(1))}
                  onChange={(event) => setSettings({ pcaVarianceThreshold: Number(event.target.value) / 100 })}
                />
              </label>
            ) : null}
          </div>
        ) : null}

        {kind === "regression" ? (
          <div className="nd-pca-settings nd-ols-settings wide">
            <label htmlFor="nd-calculation-regression-type">Regression type
              <select
                id="nd-calculation-regression-type"
                value={processRegression ? "process" : logisticRegression ? "logistic" : "ols"}
                onChange={(event) => {
                  const regressionType = event.target.value as "ols" | "logistic" | "process";
                  setSettings(nativeRegressionTypeSettingsPatch(regressionType));
                }}
              >
                <option value="ols">Ordinary least squares</option>
                <option value="logistic">Binary logistic (outcome coded 0/1)</option>
                <option value="process">Graph-defined Path Analysis / PROCESS</option>
              </select>
            </label>
            {processRegression ? (
              <NativeProcessSetup
                settings={settings}
                setSettings={setSettings}
                numericColumns={olsNumericColumns}
              />
            ) : <><label htmlFor="nd-calculation-regression-outcome">Outcome variable
              <select
                id="nd-calculation-regression-outcome"
                required
                value={selectedOlsOutcome}
                onChange={(event) => {
                  const outcome = event.target.value;
                  setSettings({
                    regressionOutcome: outcome || null,
                    regressionPredictors: selectedOlsPredictors.filter((item) => item !== outcome).join(",") || null,
                    regressionControls: selectedOlsControls.filter((item) => item !== outcome).join(",") || null,
                  });
                }}
              >
                <option value="">Select one numeric outcome</option>
                {olsNumericColumns.map((variable) => <option key={variable} value={variable}>{variable}</option>)}
              </select>
            </label>
            <fieldset className="nd-pca-variables">
              <legend>Predictors ({selectedOlsPredictors.length} selected)</legend>
              <div className="nd-pca-variable-list">
                {olsNumericColumns.map((variable) => (
                  <label key={variable}>
                    <input
                      type="checkbox"
                      checked={selectedOlsPredictorSet.has(variable)}
                      disabled={variable === selectedOlsOutcome || selectedOlsControlSet.has(variable) || (!selectedOlsPredictorSet.has(variable) && selectedOlsPredictors.length + selectedOlsControls.length >= regressionTermLimit)}
                      onChange={(event) => setOlsRoleSelected("predictor", variable, event.target.checked)}
                    />
                    <span>{variable}</span>
                  </label>
                ))}
              </div>
            </fieldset>
            <fieldset className="nd-pca-variables">
              <legend>Controls ({selectedOlsControls.length} selected, optional)</legend>
              <div className="nd-pca-variable-list">
                {olsNumericColumns.map((variable) => (
                  <label key={variable}>
                    <input
                      type="checkbox"
                      checked={selectedOlsControlSet.has(variable)}
                      disabled={variable === selectedOlsOutcome || selectedOlsPredictorSet.has(variable) || (!selectedOlsControlSet.has(variable) && selectedOlsPredictors.length + selectedOlsControls.length >= regressionTermLimit)}
                      onChange={(event) => setOlsRoleSelected("control", variable, event.target.checked)}
                    />
                    <span>{variable}</span>
                  </label>
                ))}
              </div>
            </fieldset>
            </>}
            <label className="wide" htmlFor="nd-calculation-regression-bootstrap">
              <span>Bootstrap inference</span>
              <select
                id="nd-calculation-regression-bootstrap"
                value={regressionBootstrap ? "enabled" : "off"}
                onChange={(event) => setSettings(event.target.value === "enabled"
                  ? {
                      regressionBootstrap: true,
                      bootstrapSamples: NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.default,
                      studentizedInnerSamples: 0,
                      permutationSamples: 0,
                      confidenceLevel: 0.95,
                    }
                  : { regressionBootstrap: false, bootstrapSamples: 0, workers: 1 })}
              >
                <option value="off">Off</option>
                <option value="enabled">Case-resampling bootstrap</option>
              </select>
            </label>
            {regressionBootstrap ? (
              <>
                <label htmlFor="nd-calculation-regression-bootstrap-samples">Bootstrap samples
                  <input
                    id="nd-calculation-regression-bootstrap-samples"
                    type="number"
                    min={NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum}
                    max={NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum}
                    step={1}
                    value={settings.bootstrapSamples}
                    onChange={(event) => setSettings({ bootstrapSamples: Number(event.target.value) })}
                  />
                </label>
                <label htmlFor="nd-calculation-regression-bootstrap-workers">Parallel workers
                  <input
                    id="nd-calculation-regression-bootstrap-workers"
                    type="number"
                    min={NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.minimum}
                    max={NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.maximum}
                    value={settings.workers}
                    onChange={(event) => setSettings({ workers: Number(event.target.value) })}
                  />
                </label>
              </>
            ) : null}
            {logisticRegression ? (
              <div
                id="nd-calculation-logistic-profile"
                className="nd-setting-note wide"
                role="status"
                aria-live="polite"
                aria-busy={logisticProfileState.status === "loading"}
              >
                <span>Complete-dataset outcome profile</span>
                <strong>
                  {logisticProfileState.status === "ready" && logisticProfileAssessment?.profile
                    ? `${logisticProfileAssessment.profile.completeCases} complete cases: ${logisticProfileAssessment.profile.zeroCases} class 0 and ${logisticProfileAssessment.profile.oneCases} class 1; ${logisticProfileAssessment.profile.omittedRows} omitted by listwise deletion`
                    : logisticProfileState.status === "loading"
                      ? <><LoaderCircle className="nd-spin" size={14} aria-hidden="true" /> Profiling bounded row pages...</>
                      : logisticProfileState.status === "failed"
                        ? "Profile unavailable; review the blocking message below."
                        : "Choose a numeric outcome and predictors to verify exact 0/1 coding across every row."}
                </strong>
              </div>
            ) : null}
            {processRegression ? (
              <>
              <div
                id="nd-calculation-process-profile"
                className="nd-setting-note wide"
                role="status"
                aria-live="polite"
                aria-busy={processProfileState.status === "loading"}
              >
                <span>Complete-dataset PROCESS profile</span>
                <strong>
                  {processProfileState.status === "ready" && processProfileAssessment?.profile
                    ? `${processProfileAssessment.profile.completeCases} global listwise-complete cases; ${processProfileAssessment.profile.omittedRows} rows omitted; ${processProfileAssessment.equationTermCounts.length} OLS equations verified`
                    : processProfileState.status === "loading"
                      ? <><LoaderCircle className="nd-spin" size={14} aria-hidden="true" /> Profiling bounded row pages...</>
                      : processProfileState.status === "failed"
                        ? "Profile unavailable; review the blocking message below."
                        : "Complete the graph to verify numeric roles, exact 0/1 moderator coding, constants, and equation sample size across every row."}
                </strong>
              </div>
              {processProfileState.status === "failed" ? (
                <button
                  id="nd-calculation-process-profile-retry"
                  type="button"
                  aria-describedby="nd-calculation-process-profile"
                  onClick={retryProcessProfile}
                >Retry PROCESS data profile</button>
              ) : null}
              </>
            ) : null}
          </div>
        ) : null}

        {resampling ? <label htmlFor="nd-calculation-seed">Seed
          <input
            id="nd-calculation-seed"
            type="number"
            min={0}
            max={4_294_967_295}
            value={settings.seed}
            onChange={(event) => setSettings({ seed: Number(event.target.value) })}
          />
        </label> : null}
        {kind === "pls_bootstrap" || kind === "plsc_bootstrap" || kind === "pls_permutation" || kind === "pls_posthoc_technical_minimum_sample_size" || cbsemBootstrap ? (
            <label htmlFor="nd-calculation-workers">Parallel workers
              <input
                id="nd-calculation-workers"
                type="number"
                min={1}
                max={cbsemBoundedLabsInterval ? NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.workers.maximum : 64}
                value={settings.workers}
                onChange={(event) => setSettings({ workers: Number(event.target.value) })}
              />
            </label>
        ) : null}

        {kind === "wpls" ? (
          <>
            <label htmlFor="nd-calculation-case-weight">Case-weight variable
              <select
                id="nd-calculation-case-weight"
                value={caseWeightColumn}
                onChange={(event) => setSettings({ caseWeightColumn: event.target.value || null })}
              >
                <option value="">Select a numeric variable</option>
                {!selectedWeightIsEligible ? (
                  <option value={caseWeightColumn} disabled>{caseWeightColumn} (not numeric)</option>
                ) : null}
                {numericColumns.map((column) => <option key={column} value={column}>{column}</option>)}
              </select>
            </label>
          </>
        ) : null}

        {kind === "cta_pls" ? (
          <div className="nd-setting-note wide" id="nd-calculation-cta-pls-scope">
            <span>Eligible indicator blocks</span>
            <strong>{ctaPlsBlocks.length
              ? ctaPlsBlocks.map((block) => `${block.constructLabel}: ${block.indicators.length} indicators, ${block.tetrads} tetrads`).join("; ")
              : "None - assign at least four indicators to one ordinary construct"}</strong>
          </div>
        ) : null}
      </div>
    </fieldset>
  );
}

function RunProgress({
  methodLabel,
  monitor,
  active = false,
}: {
  methodLabel: string;
  monitor: RunMonitorState;
  active?: boolean;
}) {
  const hasTotal = Number.isFinite(monitor.totalUnits) && monitor.totalUnits > 0;
  const progress = hasTotal
    ? Math.min(100, Math.max(0, Math.round((monitor.completedUnits / monitor.totalUnits) * 100)))
    : 0;
  const stateLabel = nativeCalculationPhaseLabel(monitor.phase, monitor.status);

  return (
    <section
      className={`nd-run-progress ${monitor.status}`}
      aria-label="Calculation status"
      aria-live="polite"
      aria-busy={active}
    >
      <div>
        <strong>{stateLabel}</strong>
        {hasTotal ? <span>{progress}%</span> : null}
      </div>
      {hasTotal ? (
        <progress max={100} value={progress} aria-label={`${methodLabel} progress`}>{progress}%</progress>
      ) : null}
      {monitor.message ? <p>{monitor.message}</p> : null}
      {monitor.error ? <p className="error" role="alert">{monitor.error}</p> : null}
      {monitor.logs.length > 0 ? (
        <details open={!active && monitor.status === "failed"}>
          <summary>Calculation log ({monitor.logs.length})</summary>
          <ol>
            {monitor.logs.map((entry) => (
              <li key={entry.id}>
                <time dateTime={entry.timestamp}>{formatLogTime(entry.timestamp)}</time>
                <span>{entry.message}</span>
              </li>
            ))}
          </ol>
        </details>
      ) : null}
    </section>
  );
}

function formatLogTime(timestamp: string): string {
  const parsed = new Date(timestamp);
  return Number.isNaN(parsed.getTime()) ? "" : parsed.toLocaleTimeString();
}
