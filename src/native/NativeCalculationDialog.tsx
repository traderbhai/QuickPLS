import {
  Calculator,
  ChartScatter,
  CheckCircle2,
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
import { isNativeDesktop, profileNativeDatasetGroups } from "../services/projectService";
import {
  filterNativeAnalysisCatalog,
  nativeAnalysisCatalogItem,
  nativeAnalysisStartLabel,
  type NativeAnalysisCategoryId,
  type NativeWorkbenchAnalysisKind,
} from "./nativeAnalysisCatalog";
import {
  NATIVE_PREDICTION_BENCHMARK_DESCRIPTION,
  NATIVE_PREDICTION_CVPAT_DESCRIPTION,
  NATIVE_PREDICTION_PLAN_DESCRIPTION,
  NATIVE_PREDICTION_TARGET_DESCRIPTION,
} from "./nativeCalculationMode";
import { NATIVE_ANALYSIS_RECIPE_BOUNDS } from "./nativeAnalysisRecipe";
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
  NATIVE_NCA_SCOPE_NOTE,
  nativeNcaNumericColumns,
} from "./nativeNca";
import {
  NATIVE_PCA_MAX_VARIABLES,
  NATIVE_PCA_SCOPE_NOTE,
  nativePcaNumericColumns,
  nativePcaSelectedVariables,
} from "./nativePca";
import {
  NATIVE_OLS_MAX_TERMS,
  NATIVE_OLS_SCOPE_NOTE,
  nativeOlsCsvValues,
  nativeOlsNumericColumns,
} from "./nativeOls";
import { NATIVE_GSCA_SCOPE_NOTE } from "./nativeGsca";

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
  start: () => void;
  cancel: () => void;
  close: () => void;
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
  wpls: Scale,
  gsca: Calculator,
  cca: Search,
  ipma: Target,
  cbsem: Calculator,
  pls_bootstrap: RotateCcw,
  pls_permutation: Shuffle,
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

const optionId = (kind: NativeWorkbenchAnalysisKind) => `nd-calculation-method-${kind}`;
const optionDescriptionId = (kind: NativeWorkbenchAnalysisKind) => `${optionId(kind)}-description`;
const panelTitleId = (kind: NativeWorkbenchAnalysisKind) => `nd-calculation-panel-${kind}-title`;

export function scrollNativeMethodOptionIntoView(
  option: Pick<HTMLButtonElement, "scrollIntoView"> | null | undefined,
) {
  option?.scrollIntoView({ behavior: "auto", block: "nearest", inline: "nearest" });
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
}: NativeCalculationDialogProps) {
  const [query, setQuery] = useState("");
  const [focusedKind, setFocusedKind] = useState<NativeWorkbenchAnalysisKind>(kind);
  const [groupProfileState, setGroupProfileState] = useState<GroupProfileState>({ status: "idle", profile: null, error: null });
  const searchRef = useRef<HTMLInputElement>(null);
  const optionRefs = useRef<Partial<Record<NativeWorkbenchAnalysisKind, HTMLButtonElement | null>>>({});
  const filteredMethods = useMemo(() => filterNativeAnalysisCatalog(query), [query]);
  const selectedMethod = nativeAnalysisCatalogItem(kind);
  const running = ACTIVE_RUN_STATUSES.has(runMonitor.status);
  const retry = RETRY_RUN_STATUSES.has(runMonitor.status);
  const rovingKind = filteredMethods.some((method) => method.kind === focusedKind)
    ? focusedKind
    : filteredMethods.some((method) => method.kind === kind)
      ? kind
      : filteredMethods[0]?.kind;
  const groupColumn = kind === "mga" ? settings.groupColumn?.trim() ?? "" : "";
  const analysisColumnKey = useMemo(
    () => [...new Set(analysisColumns)].sort().join("\u0000"),
    [analysisColumns],
  );
  const stableAnalysisColumns = useMemo(
    () => analysisColumnKey ? analysisColumnKey.split("\u0000") : [],
    [analysisColumnKey],
  );
  const ipmaTargetOptions = useMemo(() => nativeIpmaTargetOptions(nodes, edges), [edges, nodes]);
  const groupProfileAssessment = useMemo(
    () => nativeMgaProfileAssessment(groupProfileState.profile, settings),
    [groupProfileState.profile, settings],
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
  const canStart = readiness.canRun && (kind !== "mga" || (groupProfileState.status === "ready" && groupProfileAssessment.canRun));

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
    if (kind !== "ipma") return;
    const currentTarget = settings.ipmaTargets?.trim() ?? "";
    const selectedTarget = ipmaTargetOptions.some((option) => option.id === currentTarget)
      ? currentTarget
      : ipmaTargetOptions.length === 1
        ? ipmaTargetOptions[0].id
        : null;
    if (selectedTarget !== (settings.ipmaTargets ?? null)) setSettings({ ipmaTargets: selectedTarget });
  }, [ipmaTargetOptions, kind, setSettings, settings.ipmaTargets]);

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
        if (!running && canStart) start();
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
                      aria-describedby={optionDescriptionId(method.kind)}
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
                        <small id={optionDescriptionId(method.kind)}>{method.description}</small>
                      </span>
                    </button>
                  );
                })}
              </div>
            );
          })}
          {filteredMethods.length === 0 ? (
            <p className="nd-method-empty">No methods match “{query.trim()}”.</p>
          ) : null}
        </div>
      </aside>

      <div className="nd-dialog-content">
        {running ? (
          <RunProgress methodLabel={selectedMethod.label} monitor={runMonitor} active />
        ) : (
          <>
            <section
              id="nd-calculation-panel"
              className="nd-method-settings-panel"
              role="region"
              aria-labelledby={panelTitleId(kind)}
            >
              <header className="nd-method-settings-header">
                <h3 id={panelTitleId(kind)}>{selectedMethod.label}</h3>
                <p>{selectedMethod.description}</p>
              </header>
              <MethodSettings
                kind={kind}
                settings={settings}
                setSettings={setSettings}
                dataset={dataset}
                analysisColumns={stableAnalysisColumns}
                nodes={nodes}
                edges={edges}
                groupProfileState={groupProfileState}
                groupProfileAssessment={groupProfileAssessment}
              />
            </section>

            {!readiness.canRun || groupProfileBlockers.length ? (
              <div className="nd-blocker" role="alert">
                <strong>Cannot start this calculation</strong>
                <ul>
                  {readiness.blockers.map((blocker) => <li key={blocker.id}>{blocker.detail}</li>)}
                  {groupProfileBlockers.map((blocker) => <li key={blocker}>{blocker}</li>)}
                </ul>
              </div>
            ) : null}

            {runMonitor.status !== "idle" ? (
              <RunProgress methodLabel={selectedMethod.label} monitor={runMonitor} />
            ) : null}
          </>
        )}
      </div>

      <footer>
        {running ? (
          <button type="button" onClick={cancel} disabled={runMonitor.status === "cancelling"}>
            {runMonitor.status === "cancelling" ? "Cancelling…" : "Cancel calculation"}
          </button>
        ) : (
          <>
            <button type="button" onClick={close}>Close</button>
            <button className="primary" type="submit" disabled={!canStart}>
              <Play size={14} aria-hidden="true" />
              {nativeAnalysisStartLabel(kind, retry)}
            </button>
          </>
        )}
      </footer>
    </form>
  );
}

function MethodSettings({
  kind,
  settings,
  setSettings,
  dataset,
  analysisColumns,
  groupProfileState,
  groupProfileAssessment,
  nodes,
  edges,
}: Pick<NativeCalculationDialogProps, "kind" | "settings" | "setSettings" | "dataset" | "analysisColumns" | "nodes" | "edges"> & {
  groupProfileState: GroupProfileState;
  groupProfileAssessment: NativeMgaProfileAssessment;
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
  const resampling = kind === "pls_bootstrap" || kind === "pls_permutation" || kind === "mga" || kind === "predict" || kind === "nca";
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
  const initializedPcaSelection = useRef(false);
  const initializedOlsSelection = useRef(false);
  useEffect(() => {
    if (kind !== "pca" || initializedPcaSelection.current) return;
    initializedPcaSelection.current = true;
    if (!selectedPcaVariables.length && pcaNumericColumns.length >= 2) {
      setSettings({ pcaVariables: pcaNumericColumns.slice(0, NATIVE_PCA_MAX_VARIABLES).join(",") });
    }
  }, [kind, pcaNumericColumns, selectedPcaVariables.length, setSettings]);
  useEffect(() => {
    if (kind !== "regression" || initializedOlsSelection.current) return;
    initializedOlsSelection.current = true;
    if (!selectedOlsOutcome && !selectedOlsPredictors.length && olsNumericColumns.length >= 2) {
      setSettings({
        regressionType: "ols",
        regressionOutcome: olsNumericColumns[0],
        regressionPredictors: olsNumericColumns[1],
        regressionControls: null,
        robustSe: "hc3",
      });
    }
  }, [kind, olsNumericColumns, selectedOlsOutcome, selectedOlsPredictors.length, setSettings]);

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
        {kind === "nca" || kind === "pca" || kind === "regression" ? (
          <div className="nd-setting-note">
            <span>Calculation basis</span>
            <strong>{kind === "pca" ? "Correlation matrix (fixed)" : kind === "regression" ? "Raw-value OLS with intercept (fixed)" : "Observed variables (fixed)"}</strong>
          </div>
        ) : kind === "ipma" || kind === "mga" || kind === "cbsem" || kind === "gsca" ? (
          <div className="nd-setting-note">
            <span>Weighting scheme</span>
            <strong>Path weighting (fixed)</strong>
          </div>
        ) : <label htmlFor="nd-calculation-weighting">Weighting scheme
          <select
            id="nd-calculation-weighting"
            value={settings.weightingScheme ?? "path"}
            onChange={(event) => setSettings({
              weightingScheme: event.target.value as NonNullable<AnalysisUiSettings["weightingScheme"]>,
            })}
          >
            <option value="path">Path weighting</option>
            <option value="factor">Factor weighting</option>
            <option value="pca" disabled={kind === "plsc" || kind === "wpls" || kind === "cca"}>PCA weighting</option>
          </select>
        </label>}

        {kind === "nca" || kind === "regression" ? (
          <div className="nd-setting-note">
            <span>Variable data</span>
            <strong>{kind === "regression" ? "Unstandardized numeric values (fixed)" : "Observed numeric values (fixed)"}</strong>
          </div>
        ) : kind === "pca" ? (
          <div className="nd-setting-note">
            <span>Variable data</span>
            <strong>Standardized numeric values (fixed)</strong>
          </div>
        ) : kind === "wpls" || kind === "cca" || kind === "ipma" || kind === "mga" || kind === "cbsem" || kind === "gsca" ? (
          <div className="nd-setting-note">
            <span>Result data</span>
            <strong>Standardized (fixed)</strong>
          </div>
        ) : (
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
        )}

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

        {kind === "pls_bootstrap" ? (
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
          </>
        ) : null}

        {kind === "pls_permutation" ? (
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

            {groupProfileState.status === "loading" ? <p role="status">Reading complete-dataset group counts…</p> : null}
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
            <div className="nd-setting-note">
              <span>Test</span>
              <strong>Two-tailed; Group A − Group B</strong>
            </div>
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
            <div className="nd-setting-note wide">
              <span>Measurement invariance</span>
              <strong>Step 2 composition and Step 3 pooled-score means and variances are tested with the same deterministic permutations.</strong>
            </div>
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
          <label className="wide" htmlFor="nd-calculation-cbsem-model-type">Model type
            <select
              id="nd-calculation-cbsem-model-type"
              value={settings.cbsemModelType ?? "sem"}
              onChange={(event) => setSettings({
                cbsemModelType: event.target.value as NonNullable<AnalysisUiSettings["cbsemModelType"]>,
              })}
            >
              <option value="sem">Structural equation model (paths required)</option>
              <option value="cfa">Confirmatory factor analysis (no paths)</option>
            </select>
          </label>
        ) : null}

        {kind === "gsca" ? (
          <>
            <div className="nd-setting-note wide" id="nd-calculation-gsca-estimator">
              <span>Estimator</span>
              <strong>Joint global least-squares alternating least squares; fixed +1 initialization</strong>
            </div>
            <div className="nd-setting-note wide" id="nd-calculation-gsca-scope">
              <span>Validated scope</span>
              <strong>{NATIVE_GSCA_SCOPE_NOTE}</strong>
            </div>
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
            <div className="nd-setting-note wide">
              <span>Validated scope</span>
              <strong>{NATIVE_NCA_SCOPE_NOTE}</strong>
            </div>
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
            <div className="nd-setting-note wide">
              <span>Validated scope</span>
              <strong>{NATIVE_PCA_SCOPE_NOTE}</strong>
            </div>
          </div>
        ) : null}

        {kind === "regression" ? (
          <div className="nd-pca-settings nd-ols-settings wide">
            <label htmlFor="nd-calculation-ols-outcome">Outcome variable
              <select
                id="nd-calculation-ols-outcome"
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
                      disabled={variable === selectedOlsOutcome || selectedOlsControlSet.has(variable) || (!selectedOlsPredictorSet.has(variable) && selectedOlsPredictors.length + selectedOlsControls.length >= NATIVE_OLS_MAX_TERMS)}
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
                      disabled={variable === selectedOlsOutcome || selectedOlsPredictorSet.has(variable) || (!selectedOlsControlSet.has(variable) && selectedOlsPredictors.length + selectedOlsControls.length >= NATIVE_OLS_MAX_TERMS)}
                      onChange={(event) => setOlsRoleSelected("control", variable, event.target.checked)}
                    />
                    <span>{variable}</span>
                  </label>
                ))}
              </div>
            </fieldset>
            <div className="nd-setting-note">
              <span>Uncertainty</span>
              <strong>HC3 robust SE; two-sided 95% CI (fixed)</strong>
            </div>
            <div className="nd-setting-note wide">
              <span>Validated scope</span>
              <strong>{NATIVE_OLS_SCOPE_NOTE}</strong>
            </div>
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
        {kind === "pls_bootstrap" || kind === "pls_permutation" ? (
            <label htmlFor="nd-calculation-workers">Parallel workers
              <input
                id="nd-calculation-workers"
                type="number"
                min={1}
                max={64}
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
            <div className="nd-setting-note wide">
              <span>Case weights</span>
              <strong>Positive finite values; complete column validated at start</strong>
            </div>
          </>
        ) : null}

        {kind === "plsc" ? (
          <div className="nd-setting-note wide">
            <span>Validated scope</span>
            <strong>Reflective constructs with at least two indicators each</strong>
          </div>
        ) : null}

        {kind === "cca" ? (
          <div className="nd-setting-note wide">
            <span>Validated scope</span>
            <strong>Reflective composite path model; descriptive residual diagnostics only</strong>
          </div>
        ) : null}

        {kind === "cbsem" ? (
          <>
            <div className="nd-setting-note wide" id="nd-calculation-cbsem-estimator">
              <span>Estimator and identification</span>
              <strong>Maximum likelihood; first loading fixed to 1 for each latent factor</strong>
            </div>
            <div className="nd-setting-note wide" id="nd-calculation-cbsem-scope">
              <span>Validated scope</span>
              <strong>Single-group reflective raw-data CFA or recursive SEM; listwise-standardized indicators; no mean structure, bootstrap, robust/ordinal/FIML estimator, or invariance testing</strong>
            </div>
          </>
        ) : null}

        {kind === "ipma" ? (
          <>
            <div className="nd-setting-note wide">
              <span>Reported constructs</span>
              <strong>Direct and indirect structural predecessors only; the target and unrelated constructs are omitted</strong>
            </div>
            <div className="nd-setting-note wide">
              <span>Performance scope</span>
              <strong>0–100 observed-range scaling of standardized composite scores; no theoretical-range correction</strong>
            </div>
          </>
        ) : null}

        {kind === "predict" ? (
          <>
            <div id="nd-calculation-prediction-plan" className="nd-setting-note wide">
              <span>Validation plan</span>
              <strong>{NATIVE_PREDICTION_PLAN_DESCRIPTION}</strong>
            </div>
            <div id="nd-calculation-prediction-targets" className="nd-setting-note wide">
              <span>Prediction targets</span>
              <strong>{NATIVE_PREDICTION_TARGET_DESCRIPTION}</strong>
            </div>
            <div id="nd-calculation-prediction-benchmarks" className="nd-setting-note wide">
              <span>Benchmarks</span>
              <strong>{NATIVE_PREDICTION_BENCHMARK_DESCRIPTION}</strong>
            </div>
            <div id="nd-calculation-prediction-cvpat" className="nd-setting-note wide">
              <span>CVPAT scope</span>
              <strong>{NATIVE_PREDICTION_CVPAT_DESCRIPTION}</strong>
            </div>
          </>
        ) : null}

        {kind === "pls_algorithm" ? (
          <div className="nd-setting-note">
            <span>Execution</span>
            <strong>Deterministic single worker</strong>
          </div>
        ) : null}

        <div className="nd-setting-note">
          <span>Missing data</span>
          <strong>Listwise deletion</strong>
        </div>
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
