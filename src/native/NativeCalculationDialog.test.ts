import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset, RunMonitorState } from "../types";
import {
  capabilityAvailabilityV2,
  EXPERIMENTAL_LABS_WARNING,
} from "../domain/capabilitySurfaceV2";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import type { MethodCapabilityRegistryReaderV2 } from "../domain/methodCapabilityRegistryV2";
import { defaultGeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import { convertLegacyBasicModelV4, type SemModelV4 } from "../domain/semModelV4";
import { resolveUnifiedSemCalculationV1 } from "../domain/unifiedSemCalculationV1";
import {
  default as NativeCalculationDialog,
  dispatchNativeCalculationStartV1,
  NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS,
  nativeCalculationCatalogEntriesV2,
  nativeExperimentalWarningSessionKeys,
  nativeLogisticOutcomeBlockingMessageV1,
  nativePrimaryCalculationBlockerV1,
  nativeRegressionTypeSettingsPatch,
  nativeNumericCaseWeightColumns,
  nativeVisibleCalculationCatalogV2,
  retryNativeProcessProfileState,
  shouldStartNativeProcessProfile,
  scrollNativeMethodOptionIntoView,
  unifiedInteractionSummaryV1,
} from "./NativeCalculationDialog";
import {
  NATIVE_ANALYSIS_CATALOG,
  NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1,
  type NativeWorkbenchAnalysisKind,
} from "./nativeAnalysisCatalog";

function satisfiesNumberInputConstraints(
  value: number,
  constraints: { min: number; max: number; step: number },
): boolean {
  return Number.isInteger(value)
    && value >= constraints.min
    && value <= constraints.max
    && (value - constraints.min) % constraints.step === 0;
}

const settings: AnalysisUiSettings = {
  method: "pls_pm",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 7,
  workers: 1,
  confidenceLevel: 0.95,
};

const runMonitor: RunMonitorState = {
  status: "idle",
  phase: "",
  message: "",
  completedUnits: 0,
  totalUnits: 0,
  startedAt: null,
  completedAt: null,
  activeJobId: null,
  lastRunId: null,
  error: null,
  logs: [],
};

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1"] } },
  { id: "y", position: { x: 250, y: 0 }, data: { label: "Retention", shortName: "RET", mode: "reflective", indicators: ["y1"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];

/**
 * Presentation-only tests exercise settings panels against an explicit Labs
 * fixture. Product-surface tests also prove that the established implemented
 * catalogue remains usable in Labs even when Registry qualification is lower.
 */
const executableLabsRegistry: MethodCapabilityRegistryReaderV2 = {
  quickPlsCell(cellId) {
    return capabilityRegistryV2.quickPlsCell(cellId).map((match) => ({
      ...match,
      cell: {
        ...match.cell,
        coverage_state: match.cell.coverage_state === "intentionally_excluded" ? "intentionally_excluded" : "partial",
        evidence_state: match.cell.coverage_state === "intentionally_excluded" ? "absent" : "engine_only",
        surface: match.cell.coverage_state === "intentionally_excluded" ? "legacy" : "labs",
      },
    }));
  },
  availability(capabilityId, cellId, experimentalLabsEnabled) {
    const match = this.quickPlsCell(cellId).find((candidate) => candidate.row.capability_id === capabilityId);
    if (!match) throw new Error(`Missing test capability cell ${capabilityId}::${cellId}`);
    return capabilityAvailabilityV2(match.cell, experimentalLabsEnabled);
  },
};

function renderReadyDialog(
  kind: NativeWorkbenchAnalysisKind,
  methodSettings: AnalysisUiSettings,
  analysisNodes: Array<Node<ConstructData>> = nodes,
  experimentalLabsEnabled = true,
  openMethodDetails?: () => void,
  registryUnavailableReason?: string | null,
  capabilityRegistry: MethodCapabilityRegistryReaderV2 = executableLabsRegistry,
): string {
  return renderToStaticMarkup(createElement(NativeCalculationDialog, {
    kind,
    setKind: () => undefined,
    settings: methodSettings,
    setSettings: () => undefined,
    readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
    runMonitor,
    dataset: { id: "study", name: "study.csv", columns: [], rows: [], missing: 0 },
    analysisColumns: [],
    nodes: analysisNodes,
    edges,
    experimentalLabsEnabled,
    openMethodDetails,
    registryUnavailableReason,
    capabilityRegistry,
    start: () => undefined,
    cancel: () => undefined,
    close: () => undefined,
  }));
}

function renderWithLiveRegistry(
  kind: NativeWorkbenchAnalysisKind,
  methodSettings: AnalysisUiSettings,
  analysisNodes: Array<Node<ConstructData>> = nodes,
): string {
  return renderReadyDialog(
    kind,
    methodSettings,
    analysisNodes,
    true,
    undefined,
    undefined,
    capabilityRegistryV2,
  );
}

const metadata = (name: string, columnType: ColumnMetadata["column_type"]): ColumnMetadata => ({
  name,
  label: null,
  column_type: columnType,
  scale_type: columnType === "numeric" ? "continuous" : "nominal",
  missing_markers: ["", "NA"],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: {},
});

describe("NativeCalculationDialog blocker presentation", () => {
  it("prioritizes scientific invalidity and unsupported scope ahead of runtime and advice", () => {
    const primary = nativePrimaryCalculationBlockerV1([
      { tier: "runtime", cause: "Desktop runtime unavailable.", correction: "Open the desktop app." },
      { tier: "advice", cause: "Review optional settings.", correction: null },
      { tier: "unsupported_scope", cause: "This topology is unsupported.", correction: "Revise the topology." },
      { tier: "scientific_invalidity", cause: "Outcome has only one class.", correction: "Recode the outcome." },
    ]);

    expect(primary).toEqual({
      tier: "scientific_invalidity",
      cause: "Outcome has only one class.",
      correction: "Recode the outcome.",
    });
  });

  it("keeps equal-tier ordering stable and ignores empty causes", () => {
    expect(nativePrimaryCalculationBlockerV1([
      { tier: "scientific_invalidity", cause: "   ", correction: null },
      { tier: "unsupported_scope", cause: "First scope problem.", correction: "First correction." },
      { tier: "unsupported_scope", cause: "Second scope problem.", correction: "Second correction." },
    ])).toEqual({
      tier: "unsupported_scope",
      cause: "First scope problem.",
      correction: "First correction.",
    });
  });

  it("produces a targeted 0/1 correction from the complete-dataset logistic profile", () => {
    const blocker = nativeLogisticOutcomeBlockingMessageV1({
      datasetId: "dataset",
      datasetFingerprint: "sha256:dataset",
      outcome: "converted",
      predictors: ["score"],
      controls: [],
      expectedRows: 36,
      scannedRows: 36,
      completeCases: 36,
      omittedRows: 0,
      zeroCases: 0,
      oneCases: 0,
      invalidOutcomeRows: 36,
      constantTerms: [],
    });

    expect(blocker).toMatchObject({
      tier: "scientific_invalidity",
      correctionTargetId: "nd-calculation-regression-outcome",
      correctionActionLabel: "Change outcome",
    });
    expect(blocker?.cause).toContain("Detected: 0 (0), 1 (0), outside 0/1 (36)");
    expect(blocker?.correction).toContain("numeric 0/1");
  });
});

describe("NativeCalculationDialog contracts", () => {
  it("dispatches strict execution through the unified event seam and retains legacy fallback", () => {
    const strictModel = convertLegacyBasicModelV4({
      id: "model:calculate-event",
      name: "Calculate event",
      constructs: ["x", "m", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: [`${id}1`, `${id}2`],
      })),
      paths: [{ source: "x", target: "m" }, { source: "m", target: "y" }],
    }, "pls_composite");
    const plan = resolveUnifiedSemCalculationV1({
      method: "pls_algorithm",
      context: {
        authorityKey: "authority:event",
        model: strictModel,
        config: defaultGeneralSemConfigV1(),
      },
      bootstrap: { resamples: 500, seed: 7, confidenceLevel: 0.95 },
    });
    const onAction = vi.fn();
    const legacyStart = vi.fn();

    expect(dispatchNativeCalculationStartV1(plan, onAction, legacyStart)).toBe("unified_sem");
    expect(onAction).toHaveBeenCalledWith({ kind: "start", plan });
    expect(legacyStart).not.toHaveBeenCalled();

    expect(dispatchNativeCalculationStartV1(plan, undefined, legacyStart)).toBe("unavailable");
    expect(legacyStart).not.toHaveBeenCalled();

    expect(dispatchNativeCalculationStartV1(null, undefined, legacyStart)).toBe("legacy");
    expect(legacyStart).toHaveBeenCalledOnce();
  });

  it("offers Method Details beside the selected setup when the desktop host provides it", () => {
    const markup = renderReadyDialog("pls_algorithm", settings, nodes, true, vi.fn());
    expect(markup).toContain('class="nd-method-details-link"');
    expect(markup).toContain(">Method Details</button>");
  });

  it("preserves seeded predictors across fast non-PROCESS type switches and clears them only for PROCESS", () => {
    const autoSeeded = {
      ...settings,
      regressionType: "ols" as const,
      regressionOutcome: "outcome",
      regressionPredictors: "predictor",
      regressionControls: null,
    };
    const logisticPatch = nativeRegressionTypeSettingsPatch("logistic");
    const olsPatch = nativeRegressionTypeSettingsPatch("ols");

    expect(Object.prototype.hasOwnProperty.call(logisticPatch, "regressionPredictors")).toBe(false);
    expect(Object.prototype.hasOwnProperty.call(olsPatch, "regressionPredictors")).toBe(false);
    expect({ ...autoSeeded, ...logisticPatch }).toMatchObject({
      regressionType: "logistic",
      regressionOutcome: "outcome",
      regressionPredictors: "predictor",
      robustSe: "none",
    });

    const processSettings = {
      ...autoSeeded,
      ...nativeRegressionTypeSettingsPatch("process"),
    };
    expect(processSettings.regressionPredictors).toBeNull();

    const authoredProcessSettings = {
      ...processSettings,
      regressionOutcome: "Y",
      regressionPredictors: "X,M",
    };
    expect({ ...authoredProcessSettings, ...nativeRegressionTypeSettingsPatch("logistic") }).toMatchObject({
      regressionType: "logistic",
      regressionOutcome: "Y",
      regressionPredictors: "X,M",
      robustSe: "none",
    });
  });

  it("does not reprofile an unchanged PROCESS selection after runtime-only setting edits", () => {
    const key = ["dataset", "fingerprint", "40", "scientific-selection"].join("\0");
    expect(shouldStartNativeProcessProfile(true, null, true, key, { status: "idle", key: "" })).toBe(true);
    expect(shouldStartNativeProcessProfile(true, null, true, key, { status: "loading", key })).toBe(false);
    expect(shouldStartNativeProcessProfile(true, null, true, key, { status: "ready", key })).toBe(false);
    expect(shouldStartNativeProcessProfile(true, null, true, `${key}-changed`, { status: "ready", key })).toBe(true);
    expect(shouldStartNativeProcessProfile(true, null, true, key, { status: "failed", key })).toBe(false);
    const retry = retryNativeProcessProfileState(key);
    expect(retry).toEqual({ status: "idle", key, profile: null, error: null });
    expect(shouldStartNativeProcessProfile(true, null, true, key, retry)).toBe(true);
    expect(shouldStartNativeProcessProfile(true, null, true, key, { status: "ready", key })).toBe(false);
  });

  it("reveals the selected method without moving focus or animating the catalog", () => {
    const scrollIntoView = vi.fn();

    scrollNativeMethodOptionIntoView({ scrollIntoView });
    scrollNativeMethodOptionIntoView(null);

    expect(scrollIntoView).toHaveBeenCalledOnce();
    expect(scrollIntoView).toHaveBeenCalledWith({
      behavior: "auto",
      block: "nearest",
      inline: "nearest",
    });
  });

  it("keeps the established methods plus the executable post-hoc add-on available while exposing bounded methods in Standard", () => {
    const standardEntries = nativeCalculationCatalogEntriesV2(settings, false);
    const standardVisible = nativeVisibleCalculationCatalogV2(settings, false);
    const labsVisible = nativeVisibleCalculationCatalogV2(settings, true);
    const standardKinds = ["pls_algorithm", "plsc", "wpls", "gsca", "cca", "cta_pls", "ipma", "cbsem", "pls_bootstrap", "plsc_bootstrap", "pls_permutation", "pls_posthoc_technical_minimum_sample_size", "pls_sample_size_power", "mga", "predict", "nca", "pca", "regression"];
    const expectedLabsKinds = NATIVE_ANALYSIS_CATALOG
      .map((item) => item.kind)
      .filter((kind) => standardKinds.includes(kind)
        || NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1.includes(kind as typeof NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1[number]));

    expect(standardEntries).toHaveLength(NATIVE_ANALYSIS_CATALOG.length);
    expect(standardVisible.map((entry) => entry.item.kind)).toEqual(standardKinds);
    expect(labsVisible.map((entry) => entry.item.kind)).toEqual(expectedLabsKinds);
    expect(labsVisible).toHaveLength(18);
    expect(standardEntries.filter((entry) => standardKinds.includes(entry.item.kind))).toHaveLength(standardKinds.length);
    expect(standardEntries.filter((entry) => !standardKinds.includes(entry.item.kind)).every((entry) => (
      !entry.availability.selectable && entry.availability.tier === "hidden"
    ))).toBe(true);
  });

  it("shows the scoped Standard post-hoc workflow with only its contracted bootstrap plan", () => {
    const markup = renderWithLiveRegistry("pls_posthoc_technical_minimum_sample_size", settings);

    expect(markup).toContain("Post-hoc Technical Minimum Sample Size");
    expect(markup).not.toContain('<strong>Post-hoc Technical Minimum Sample Size</strong><span class="nd-form-status nd-inline-warning" data-method-chip="experimental">Experimental</span>');
    expect(markup).not.toContain("Inference contract");
    expect(markup).toContain('id="nd-calculation-bootstrap-samples"');
    expect(markup).not.toContain('id="nd-calculation-studentized"');
    expect(markup).not.toContain("two-sided normal-reference probabilities at alpha 0.05");
  });

  it("shows point-estimate PLSc as Supported while keeping its inference add-ons separate", () => {
    const markup = renderWithLiveRegistry("plsc", { ...settings, method: "plsc" });
    const optionStart = markup.indexOf('id="nd-calculation-method-plsc"');
    const selectedOption = markup.slice(optionStart, markup.indexOf("</button>", optionStart));

    expect(markup).toMatch(/id="nd-calculation-method-plsc"[^>]*aria-selected="true"/);
    expect(markup).toContain("Consistent PLS");
    expect(selectedOption).not.toContain('data-method-chip="experimental"');
    expect(selectedOption).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain('data-experimental-warning');
    expect(markup).not.toContain('data-limited-scope-warning');
  });

  it("shows the bounded full-refit PLSc bootstrap workflow as Supported", () => {
    const markup = renderWithLiveRegistry("plsc_bootstrap", {
      ...settings,
      method: "plsc",
      bootstrapSamples: 1_000,
      permutationSamples: 0,
    });
    const optionStart = markup.indexOf('id="nd-calculation-method-plsc_bootstrap"');
    const selectedOption = markup.slice(optionStart, markup.indexOf("</button>", optionStart));

    expect(markup).toMatch(/id="nd-calculation-method-plsc_bootstrap"[^>]*aria-selected="true"/);
    expect(selectedOption).not.toContain('data-method-chip="experimental"');
    expect(selectedOption).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain("full-PLSc delete-one refit");
    expect(markup).not.toContain('data-experimental-warning');
  });

  it("routes an archived schema-3 CB-SEM bootstrap setting to the exact workspace", () => {
    const markup = renderReadyDialog("cbsem", {
      ...settings,
      method: "cbsem",
      cbsemModelType: "sem",
      cbsemBootstrapSamples: 500,
      confidenceLevel: 0.95,
    });

    expect(markup).toContain('id="nd-calculation-cbsem-archived-bootstrap"');
    expect(markup).toContain("Clear the archived bootstrap setting before running this point-estimate setup.");
    expect(markup).toContain("Clear setting");
    expect(markup).not.toContain('id="nd-calculation-cbsem-bootstrap-samples"');
  });

  it("does not route analytic-studentized CFA through schema-3 Calculate", () => {
    const markup = renderReadyDialog("cbsem", {
      ...settings,
      method: "cbsem",
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
      cbsemBootstrapInterval: "analytic_studentized_type7",
      cbsemBootstrapTestTail: "two_sided",
      workers: 12,
      confidenceLevel: 0.95,
    });

    expect(markup).toContain('id="nd-calculation-cbsem-archived-bootstrap"');
    expect(markup).not.toContain('id="nd-calculation-cbsem-bootstrap-interval"');
    expect(markup).not.toContain("Analytic studentized Type 7 (Labs)");
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
  });

  it("does not route BCa CFA through schema-3 Calculate", () => {
    const markup = renderReadyDialog("cbsem", {
      ...settings,
      method: "cbsem",
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
      cbsemBootstrapInterval: "bca_type7",
      cbsemBootstrapTestTail: "two_sided",
      workers: 12,
      confidenceLevel: 0.95,
    });

    expect(markup).toContain('id="nd-calculation-cbsem-archived-bootstrap"');
    expect(markup).not.toContain('aria-describedby="nd-calculation-cbsem-bootstrap-interval-note"');
    expect(markup).not.toContain("BCa Type 7 (Labs, complete-only)");
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
  });

  it("does not expose exact-CFA test tails in schema-3 Calculate", () => {
    const markup = renderReadyDialog("cbsem", {
      ...settings,
      method: "cbsem",
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
      cbsemBootstrapTestTail: "one_sided_greater",
      confidenceLevel: 0.95,
    });

    expect(markup).toContain('id="nd-calculation-cbsem-archived-bootstrap"');
    expect(markup).not.toContain('id="nd-calculation-cbsem-bootstrap-test-tail"');
    expect(markup).not.toContain("One-sided: parameter is greater than zero");
  });

  it("keeps point CB-SEM and the exact-CFA bootstrap route Standard without exposing schema-3 controls", () => {
    const pointEstimator = nativeCalculationCatalogEntriesV2({
      ...settings,
      cbsemBootstrapSamples: 0,
    }, true).find((entry) => entry.item.kind === "cbsem");
    const combined = nativeCalculationCatalogEntriesV2({
      ...settings,
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
    }, true).find((entry) => entry.item.kind === "cbsem");

    expect(pointEstimator?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      blocked_cell_ids: [],
      internal_reason: "all_required_cells_standard",
    });
    expect(combined?.availability).toMatchObject({
      tier: "standard",
      selectable: true,
      blocked_cell_ids: [],
      internal_reason: "all_required_cells_standard",
    });

    const markup = renderWithLiveRegistry("cbsem", {
      ...settings,
      method: "cbsem",
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
    });
    expect(markup).toContain('id="nd-calculation-method-cbsem"');
    expect(markup).toContain('id="nd-calculation-cbsem-archived-bootstrap"');
    expect(markup).not.toContain('id="nd-calculation-cbsem-bootstrap-samples"');
    const pointMarkup = renderWithLiveRegistry("cbsem", {
      ...settings,
      method: "cbsem",
      cbsemModelType: "sem",
      cbsemBootstrapSamples: 0,
    });
    const pointOptionStart = pointMarkup.indexOf('id="nd-calculation-method-cbsem"');
    const pointOption = pointMarkup.slice(pointOptionStart, pointMarkup.indexOf("</button>", pointOptionStart));
    expect(pointOption).not.toContain('data-method-chip="experimental"');
    expect(pointOption).not.toContain('data-method-chip="limited-scope"');
    expect(pointMarkup).not.toContain('data-experimental-warning');
    expect(pointMarkup).not.toContain('data-limited-scope-warning');
    expect(markup).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain('data-limited-scope-warning="true"');
    expect(markup).not.toContain("Calculation method unavailable");
    expect(markup).toMatch(/class="primary" type="submit" disabled=""/);
    expect(markup).toContain("Clear setting");
  });

  it("keeps experimental capability accounting internal while the method picker stays clean", () => {
    const entries = nativeCalculationCatalogEntriesV2(settings, true, executableLabsRegistry);
    const algorithm = entries.find((entry) => entry.item.kind === "pls_algorithm");
    const bootstrap = entries.find((entry) => entry.item.kind === "pls_bootstrap");
    const pca = entries.find((entry) => entry.item.kind === "pca");
    const shown = new Set(nativeExperimentalWarningSessionKeys(algorithm, true, new Set()));
    const bootstrapPending = nativeExperimentalWarningSessionKeys(bootstrap, true, shown);
    const pcaPending = nativeExperimentalWarningSessionKeys(pca, true, shown);

    expect([...shown]).toHaveLength(1);
    expect([...shown][0]).toMatch(/^smartpls\.pls_algorithm::qpls3\.pls\.algorithm::/);
    expect(nativeExperimentalWarningSessionKeys(algorithm, true, shown)).toEqual([]);
    expect(bootstrapPending).toHaveLength(1);
    expect(bootstrapPending[0]).toMatch(/^smartpls\.pls_bootstrapping::qpls3\.inference\.bootstrap::/);
    expect(pcaPending).toHaveLength(2);
    expect(new Set(pcaPending).size).toBe(2);
    expect(pcaPending.every((key) => key.includes("::qpls3.standalone.pca::"))).toBe(true);

    const firstMarkup = renderReadyDialog("pls_algorithm", settings);
    expect(firstMarkup).not.toContain(EXPERIMENTAL_LABS_WARNING);
    expect(firstMarkup).not.toContain('data-method-chip="experimental"');

    const repeatedMarkup = renderReadyDialog("pls_algorithm", settings, nodes, true);
    expect(repeatedMarkup).not.toContain(EXPERIMENTAL_LABS_WARNING);
    expect(repeatedMarkup).not.toContain('data-method-chip="experimental"');
  });

  it("renders an accessible requirements state and no hidden settings when Standard has no methods", () => {
    const markup = renderReadyDialog("pls_algorithm", settings, nodes, false);

    expect(markup).toContain("0 methods");
    expect(markup).toContain("No Standard methods are available yet");
    expect(markup).toContain('role="status" aria-live="polite"');
    expect(markup).toContain("Calculation method unavailable");
    expect(markup).not.toContain('id="nd-calculation-method-pls_algorithm"');
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain("PLS-SEM Algorithm");
    expect(markup).toMatch(/class="primary" type="submit" disabled=""/);
  });

  it("fails closed when the installed registry cannot be verified", () => {
    const markup = renderReadyDialog(
      "pls_algorithm",
      settings,
      nodes,
      true,
      undefined,
      "QuickPLS could not verify the installed calculation catalogue.",
    );

    expect(markup).toContain("Calculation catalogue unavailable");
    expect(markup).toContain("QuickPLS could not verify the installed calculation catalogue.");
    expect(markup).toContain("0 methods");
    expect(markup).not.toContain('id="nd-calculation-method-pls_algorithm"');
    expect(markup).toMatch(/class="primary" type="submit" disabled=""/);
  });

  it("admits every backend-valid integer resample count through the HTML number inputs", () => {
    const bootstrap = NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.bootstrap;
    const permutation = NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS.permutation;

    expect(bootstrap).toEqual({ min: 100, max: 10_000, step: 1 });
    expect(permutation).toEqual({ min: 99, max: 10_000, step: 1 });
    expect(satisfiesNumberInputConstraints(999, bootstrap)).toBe(true);
    expect(satisfiesNumberInputConstraints(100, permutation)).toBe(true);
    expect(satisfiesNumberInputConstraints(4_321, permutation)).toBe(true);
    expect(satisfiesNumberInputConstraints(99.5, permutation)).toBe(false);
    expect(satisfiesNumberInputConstraints(10_001, bootstrap)).toBe(false);
  });

  it("renders the scoped Standard bootstrap and structural-randomization workflows", () => {
    const bootstrap = renderReadyDialog("pls_bootstrap", { ...settings, bootstrapSamples: 999 });
    expect(bootstrap).toMatch(/id="nd-calculation-bootstrap-samples"[^>]*step="1"[^>]*value="999"/);
    expect(bootstrap).toMatch(/class="primary" type="submit"/);

    const permutation = renderWithLiveRegistry("pls_permutation", { ...settings, permutationSamples: 4_321 });
    expect(permutation).toContain('id="nd-calculation-method-pls_permutation"');
    expect(permutation).toMatch(/id="nd-calculation-permutations"[^>]*value="4321"/);
    const permutationOptionStart = permutation.indexOf('id="nd-calculation-method-pls_permutation"');
    const permutationOption = permutation.slice(permutationOptionStart, permutation.indexOf("</button>", permutationOptionStart));
    expect(permutationOption).not.toContain('data-method-chip="experimental"');
    expect(permutationOption).not.toContain('data-method-chip="limited-scope"');
    expect(permutation).not.toContain('data-experimental-warning="true"');
    expect(permutation).not.toContain('data-limited-scope-warning="true"');
    expect(permutation).not.toContain("Calculation method unavailable");
    expect(permutation).toMatch(/class="primary" type="submit"/);
    expect(permutation).not.toMatch(/class="primary" type="submit" disabled=""/);
  });

  it("renders the bounded prospective power v2 workflow as scoped Standard", () => {
    const powerNodes: Array<Node<ConstructData>> = [
      { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1", "x2", "x3"] } },
      { id: "y", position: { x: 250, y: 0 }, data: { label: "Retention", shortName: "RET", mode: "reflective", indicators: ["y1", "y2", "y3"] } },
    ];
    const markup = renderWithLiveRegistry("pls_sample_size_power", {
      ...settings,
      method: "pls_sample_size_power",
      weightingScheme: "path",
      preprocessing: "standardized",
      tolerance: 1e-7,
      maxIterations: 3_000,
      workers: 4,
      plsPowerScenarioIdentity: "prospective_two_construct_path",
      plsPowerPredictorConstruct: "x",
      plsPowerOutcomeConstruct: "y",
      plsPowerPredictorLoadings: "0.8,0.8,0.8",
      plsPowerOutcomeLoadings: "0.8,0.8,0.8",
      plsPowerPopulationPath: 0.30,
      plsPowerSampleSizeGrid: "50,100,150",
      plsPowerAlpha: 0.05,
      plsPowerTargetPower: 0.80,
      plsPowerMonteCarloReplicates: 250,
      plsPowerBootstrapReplicates: 199,
    }, powerNodes);

    expect(markup).toContain('id="nd-calculation-method-pls_sample_size_power"');
    for (const id of [
      "nd-calculation-pls-power-predictor-loadings",
      "nd-calculation-pls-power-outcome-loadings",
      "nd-calculation-pls-power-path",
      "nd-calculation-pls-power-grid",
      "nd-calculation-pls-power-alpha",
      "nd-calculation-pls-power-target",
      "nd-calculation-pls-power-confidence",
      "nd-calculation-pls-power-mc",
      "nd-calculation-pls-power-bootstrap",
      "nd-calculation-seed",
      "nd-calculation-pls-power-workers",
      "nd-calculation-pls-power-workload",
    ]) expect(markup).toContain(`id="${id}"`);
    expect(markup).not.toContain("Prospective Monte Carlo power");
    expect(markup).not.toContain("Calculation method unavailable");
    expect(markup).not.toContain('data-method-chip="experimental"');
    expect(markup).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain('data-experimental-warning="true"');
    expect(markup).not.toContain('data-limited-scope-warning="true"');
    expect(markup).toMatch(/class="primary" type="submit"/);
    expect(markup).not.toMatch(/class="primary" type="submit" disabled=""/);
  });

  it("offers declared numeric variables and safely inferred resident numeric variables for WPLS", () => {
    const dataset: Dataset = {
      id: "weighted",
      name: "Weighted sample",
      columns: ["declared", "text", "boolean", "inferred", "mixed", "empty"],
      rows: [
        { declared: "1", text: "one", boolean: "true", inferred: 1.5, mixed: 2, empty: null },
        { declared: "2", text: "two", boolean: "false", inferred: 2.5, mixed: "3", empty: null },
      ],
      missing: 2,
      columnMetadata: [
        metadata("declared", "numeric"),
        metadata("text", "text"),
        metadata("boolean", "boolean"),
      ],
    };

    expect(nativeNumericCaseWeightColumns(dataset)).toEqual(["declared", "inferred"]);
  });

  it("shows MICOM and permutation MGA as Supported while enforcing the bounded v4 group setup", () => {
    const markup = renderWithLiveRegistry("mga", {
      ...settings,
      method: "mga",
      groupColumn: "group",
      groupAValue: "A",
      groupBValue: "B",
      groupMethods: "micom,mga_permutation",
      groupPermutationSamples: 5_000,
      micomConfiguralConfirmed: false,
    });

    expect(markup).toContain('id="nd-calculation-method-mga"');
    expect(markup).toContain('id="nd-calculation-group-column"');
    expect(markup).toContain('id="nd-calculation-group-permutations"');
    expect(markup).toContain('id="nd-calculation-micom-configural"');
    // The selected MGA option has no availability chip. Other catalogue
    // options may independently remain Experimental or Limited scope.
    expect(markup).toMatch(/id="nd-calculation-method-mga"[\s\S]*?<strong>MICOM and Two-Group Permutation MGA<\/strong><\/span>/);
    expect(markup).not.toContain('data-limited-scope-warning="true"');
    expect(markup).not.toContain('data-experimental-warning="true"');
    expect(markup).not.toContain("Calculation method unavailable");
    expect(markup).toContain("Choose a grouping variable to load complete-dataset counts.");
    expect(markup).toMatch(/class="primary" type="submit" disabled=""/);
  });

  it("renders an accessible fixed CCA residual-diagnostics setup without ignored inference controls", () => {
    const markup = renderReadyDialog("cca", {
      ...settings,
      method: "cca",
      weightingScheme: "path",
      preprocessing: "standardized",
    });

    expect(markup).toContain('id="nd-calculation-category-assessment"');
    expect(markup).toMatch(/id="nd-calculation-method-cca"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).not.toContain('aria-describedby="nd-calculation-method-cca-description"');
    expect(markup).toContain('id="nd-calculation-panel-cca-title"');
    expect(markup).toContain("CCA composite residual diagnostics");
    expect(markup).not.toContain("Standardized (fixed)");
    expect(markup).not.toContain("Reflective composite path model; descriptive residual diagnostics only");
    expect(markup).not.toContain("Listwise deletion");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-bootstrap-samples"');
    expect(markup).not.toContain('id="nd-calculation-permutations"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain("Case-weight variable");
    expect(markup).toContain("Confirmatory Tetrad Analysis");
    expect(markup).not.toContain('id="nd-calculation-cta-pls-scope"');
  });

  it("renders an accessible CTA-PLS eligible-block summary and bounded descriptive scope", () => {
    const ctaNodes = nodes.map((node) => node.id === "x"
      ? { ...node, data: { ...node.data, mode: "formative" as const, indicators: ["x1", "x2", "x3", "x4"] } }
      : node);
    const markup = renderWithLiveRegistry("cta_pls", {
      ...settings,
      method: "cta_pls",
      weightingScheme: "path",
      preprocessing: "standardized",
    }, ctaNodes);
    const optionStart = markup.indexOf('id="nd-calculation-method-cta_pls"');
    const selectedOption = markup.slice(optionStart, markup.indexOf("</button>", optionStart));
    expect(markup).toMatch(/id="nd-calculation-method-cta_pls"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain('id="nd-calculation-cta-pls-scope"');
    expect(markup).toContain("Capability: 4 indicators, 3 tetrads");
    expect(markup).not.toContain("Descriptive sample-covariance tetrads only");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-bootstrap-samples"');
    expect(markup).not.toContain('id="nd-calculation-permutations"');
    expect(selectedOption).not.toContain('data-method-chip="experimental"');
    expect(selectedOption).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain('data-experimental-warning="true"');
    expect(markup).not.toContain('data-limited-scope-warning="true"');
  });

  it("renders one ID-backed endogenous IPMA target with fixed truthful settings", () => {
    const markup = renderReadyDialog(
      "ipma",
      {
        ...settings,
        method: "ipma",
        weightingScheme: "path",
        preprocessing: "standardized",
        ipmaTargets: "y",
      },
      nodes,
      false,
      undefined,
      undefined,
      capabilityRegistryV2,
    );

    expect(markup).toMatch(/id="nd-calculation-method-ipma"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain("Importance-Performance Map Analysis");
    expect(markup).toContain('id="nd-calculation-ipma-target"');
    expect(markup).toMatch(/id="nd-calculation-ipma-target"[^>]*required=""/);
    expect(markup).toContain('<option value="">Select one endogenous construct</option><option value="y" selected="">Retention [y]</option>');
    expect(markup).not.toContain("Path weighting (fixed)");
    expect(markup).not.toContain("Standardized (fixed)");
    expect(markup).toContain('id="nd-calculation-ipma-fixed-scope"');
    expect(markup).toContain("All direct and indirect predecessors");
    expect(markup).toContain("observed-range 0–100 performance");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-confidence"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain("Case-weight variable");
    expect(markup).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain('data-limited-scope-warning');
    expect(markup).not.toContain('data-experimental-warning');
  });

  it("renders the fixed indicator-level PLSpredict / CVPAT contract without irrelevant controls", () => {
    const markup = renderWithLiveRegistry("predict", { ...settings, method: "predict" });
    const predictOptionStart = markup.indexOf('id="nd-calculation-method-predict"');
    const predictOption = markup.slice(predictOptionStart, markup.indexOf("</button>", predictOptionStart));

    expect(markup).toMatch(/id="nd-calculation-method-predict"[^>]*aria-selected="true"/);
    expect(markup).toContain("PLSpredict / CVPAT");
    expect(markup).toContain('id="nd-calculation-prediction-plan"');
    expect(markup).toContain("10 folds × 10 repetitions (fixed)");
    expect(markup).not.toContain("Endogenous indicators are primary");
    expect(markup).not.toContain("Indicator average (IA) and Linear model (LM, where estimable)");
    expect(markup).not.toContain("one-sided test, 95% confidence; not a comparison of saved models");
    expect(markup).toContain("Start calculation");
    expect(markup).toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-confidence"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(predictOption).not.toContain('data-method-chip="experimental"');
    expect(predictOption).not.toContain('data-method-chip="limited-scope"');
    expect(markup).not.toContain('data-experimental-warning');
    expect(markup).not.toContain('data-limited-scope-warning');
  });

  it("renders a bounded observed-variable NCA setup without SEM controls or claims", () => {
    const ncaDataset: Dataset = {
      id: "nca-data",
      name: "nca.csv",
      columns: ["condition", "outcome", "segment"],
      rows: [
        { condition: 1, outcome: 2, segment: "A" },
        { condition: 2, outcome: 4, segment: "B" },
        { condition: 3, outcome: 8, segment: "B" },
      ],
      missing: 0,
      fingerprint: "sha256:nca",
      kind: "raw",
      columnMetadata: [metadata("condition", "numeric"), metadata("outcome", "numeric"), metadata("segment", "text")],
    };
    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "nca",
      experimentalLabsEnabled: true,
      capabilityRegistry: executableLabsRegistry,
      setKind: () => undefined,
      settings: {
        ...settings,
        method: "nca",
        preprocessing: "unstandardized",
        ncaX: "condition",
        ncaY: "outcome",
        ncaCeiling: "both",
        ncaPermutationSamples: 999,
      },
      setSettings: () => undefined,
      readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
      runMonitor,
      dataset: ncaDataset,
      analysisColumns: [],
      nodes: [],
      edges: [],
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));

    expect(markup).toMatch(/id="nd-calculation-method-nca"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain('id="nd-calculation-category-standalone"');
    expect(markup).toContain('id="nd-calculation-nca-x"');
    expect(markup).toContain('<option value="condition" selected="">condition</option>');
    expect(markup).toContain('id="nd-calculation-nca-y"');
    expect(markup).toContain('<option value="outcome" selected="">outcome</option>');
    expect(markup).toMatch(/id="nd-calculation-nca-permutations"[^>]*min="1"[^>]*max="10000"[^>]*step="1"[^>]*value="999"/);
    expect(markup).toContain("CE-FDH and CR-FDH");
    expect(markup).toContain('id="nd-calculation-nca-fixed-evidence"');
    expect(markup).toContain("Bottlenecks 10%–90% in 10-point steps");
    expect(markup).toContain("fixed single-worker execution");
    expect(markup).not.toContain("Multiple conditions, latent-score NCA, cIPMA");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain("Endogenous target");

    const supportedMarkup = renderWithLiveRegistry("nca", {
      ...settings,
      method: "nca",
      ncaX: "condition",
      ncaY: "outcome",
      ncaCeiling: "both",
      ncaPermutationSamples: 999,
    });
    expect(supportedMarkup).toMatch(/id="nd-calculation-method-nca"[\s\S]*?<strong>Necessary Condition Analysis<\/strong><\/span>/);
    expect(supportedMarkup).not.toContain('data-limited-scope-warning="true"');
    expect(supportedMarkup).not.toContain('data-experimental-warning="true"');
  });

  it("renders the bounded single-group CB-SEM/CFA ML setup without unsupported controls", () => {
    const markup = renderReadyDialog("cbsem", {
      ...settings,
      method: "cbsem",
      weightingScheme: "path",
      preprocessing: "standardized",
      cbsemModelType: "sem",
      cbsemMeanStructure: false,
      cbsemGroupColumn: null,
      cbsemInvarianceSteps: null,
      cbsemBootstrapSamples: 0,
    });

    expect(markup).toMatch(/id="nd-calculation-method-cbsem"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain('id="nd-calculation-category-covariance"');
    expect(markup).toContain('id="nd-calculation-cbsem-model-type"');
    expect(markup).toContain('<option value="sem" selected="">Structural equation model (paths required)</option>');
    expect(markup).not.toContain('id="nd-calculation-cbsem-estimator"');
    expect(markup).not.toContain("Maximum likelihood; first loading fixed to 1 for each latent factor");
    expect(markup).not.toContain('id="nd-calculation-cbsem-scope"');
    expect(markup).not.toContain("Single-group reflective raw-data CFA or recursive SEM");
    expect(markup).toContain('id="nd-calculation-cbsem-point-contract"');
    expect(markup).toContain("Maximum-likelihood point estimates");
    expect(markup).toContain("Start calculation");
    expect(markup).toContain('id="nd-calculation-max-iterations"');
    expect(markup).toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain('id="nd-calculation-confidence"');
  });

  it("renders GSCA as a fixed ALS component-model workflow without PLS or inference controls", () => {
    const markup = renderReadyDialog("gsca", {
      ...settings,
      method: "gsca",
      weightingScheme: "path",
      preprocessing: "standardized",
      tolerance: 1e-7,
      maxIterations: 3_000,
    });
    expect(markup).toMatch(/id="nd-calculation-method-gsca"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain('id="nd-calculation-category-component_models"');
    expect(markup).toContain('id="nd-calculation-panel-gsca-title"');
    expect(markup).not.toContain('id="nd-calculation-gsca-estimator"');
    expect(markup).not.toContain('id="nd-calculation-gsca-scope"');
    expect(markup).toContain('id="nd-calculation-gsca-fixed-summary"');
    expect(markup).toContain("Joint global least-squares ALS");
    expect(markup).not.toContain("GSCA bootstrapping, or other inference");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-seed"');

    const supportedMarkup = renderWithLiveRegistry("gsca", {
      ...settings,
      method: "gsca",
    });
    expect(supportedMarkup).toMatch(/id="nd-calculation-method-gsca"[\s\S]*?<strong>GSCA<\/strong><\/span>/);
    expect(supportedMarkup).not.toContain('data-limited-scope-warning="true"');
    expect(supportedMarkup).not.toContain('data-experimental-warning="true"');
  });

  it("renders model-free PCA variable and retention controls with fixed scientific scope", () => {
    const pcaDataset: Dataset = {
      id: "pca-data",
      name: "pca.csv",
      columns: ["a", "b", "c", "segment"],
      rows: [
        { a: 1, b: 2, c: 3, segment: "A" },
        { a: 2, b: 4, c: 1, segment: "B" },
        { a: 3, b: 1, c: 5, segment: "A" },
      ],
      missing: 0,
      fingerprint: "sha256:pca",
      kind: "raw",
      columnMetadata: [metadata("a", "numeric"), metadata("b", "numeric"), metadata("c", "numeric"), metadata("segment", "text")],
    };
    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "pca",
      experimentalLabsEnabled: true,
      capabilityRegistry: executableLabsRegistry,
      setKind: () => undefined,
      settings: {
        ...settings,
        method: "pca",
        preprocessing: "standardized",
        pcaVariables: "a,b,c",
        pcaComponentRule: "variance_threshold",
        pcaVarianceThreshold: 0.80,
      },
      setSettings: () => undefined,
      readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
      runMonitor,
      dataset: pcaDataset,
      analysisColumns: [],
      nodes: [],
      edges: [],
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));

    expect(markup).toMatch(/id="nd-calculation-method-pca"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain("Variables (3 selected)");
    expect(markup).toContain('id="nd-calculation-pca-rule"');
    expect(markup).toMatch(/id="nd-calculation-pca-threshold"[^>]*min="1"[^>]*max="99.9"[^>]*step="0.1"[^>]*value="80"/);
    expect(markup).not.toContain("Correlation matrix (fixed)");
    expect(markup).not.toContain("Standardized numeric values (fixed)");
    expect(markup).not.toContain("<span>Validated scope</span>");
    expect(markup).not.toContain("deterministic component orientation");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
  });

  it("renders bounded model-free OLS selectors and fixed HC3 inference without SEM controls", () => {
    const olsDataset: Dataset = {
      id: "ols-data",
      name: "ols.csv",
      columns: ["y", "x", "m", "group"],
      rows: [
        { y: 2, x: 1, m: 0, group: "A" },
        { y: 4, x: 2, m: 1, group: "B" },
        { y: 7, x: 3, m: 0, group: "A" },
        { y: 9, x: 4, m: 1, group: "B" },
      ],
      missing: 0,
      fingerprint: "sha256:ols",
      kind: "raw",
      columnMetadata: [metadata("y", "numeric"), metadata("x", "numeric"), metadata("m", "numeric"), metadata("group", "text")],
    };
    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "regression",
      experimentalLabsEnabled: true,
      capabilityRegistry: executableLabsRegistry,
      setKind: () => undefined,
      settings: {
        ...settings,
        method: "regression",
        preprocessing: "unstandardized",
        regressionType: "ols",
        regressionOutcome: "y",
        regressionPredictors: "x",
        regressionControls: "m",
        robustSe: "hc3",
      },
      setSettings: () => undefined,
      readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
      runMonitor,
      dataset: olsDataset,
      analysisColumns: [],
      nodes: [],
      edges: [],
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));

    expect(markup).toMatch(/id="nd-calculation-method-regression"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain('<h3 id="nd-calculation-panel-regression-title">Regression</h3>');
    expect(markup).toContain('id="nd-calculation-regression-outcome"');
    expect(markup).toContain('<option value="y" selected="">y</option>');
    expect(markup).toContain("Predictors (1 selected)");
    expect(markup).toContain("Controls (1 selected, optional)");
    expect(markup).not.toContain("HC3 robust SE; two-sided 95% CI (fixed)");
    expect(markup).not.toContain("<span>Validated scope</span>");
    expect(markup).not.toContain("Raw numeric ordinary least squares with an intercept");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');

    const supportedMarkup = renderWithLiveRegistry("regression", {
      ...settings,
      method: "regression",
      regressionType: "ols",
    });
    expect(supportedMarkup).toMatch(/id="nd-calculation-method-regression"[\s\S]*?<strong>Regression<\/strong><\/span>/);
    expect(supportedMarkup).not.toContain('data-limited-scope-warning="true"');
    expect(supportedMarkup).not.toContain('data-experimental-warning="true"');
  });

  it("renders qualified regression case-resampling controls and disclosures", () => {
    const regressionDataset: Dataset = {
      id: "bootstrap-data",
      name: "bootstrap.csv",
      columns: ["y", "x"],
      rows: Array.from({ length: 12 }, (_, index) => ({ y: index * 2 + 1, x: index + 1 })),
      rowCount: 12,
      missing: 0,
      fingerprint: "sha256:bootstrap",
      kind: "raw",
      columnMetadata: [metadata("y", "numeric"), metadata("x", "numeric")],
    };
    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "regression",
      experimentalLabsEnabled: true,
      capabilityRegistry: executableLabsRegistry,
      setKind: () => undefined,
      settings: {
        ...settings,
        method: "regression",
        preprocessing: "unstandardized",
        regressionType: "ols",
        regressionOutcome: "y",
        regressionPredictors: "x",
        regressionBootstrap: true,
        bootstrapSamples: 10_000,
        workers: 4,
        confidenceLevel: 0.95,
      },
      setSettings: () => undefined,
      readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
      runMonitor,
      dataset: regressionDataset,
      analysisColumns: [],
      nodes: [],
      edges: [],
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));

    expect(markup).toContain('id="nd-calculation-regression-bootstrap"');
    expect(markup).toContain('<option value="enabled" selected="">Case-resampling bootstrap</option>');
    expect(markup).toMatch(/id="nd-calculation-regression-bootstrap-samples"[^>]*min="99"[^>]*max="10000"[^>]*value="10000"/);
    expect(markup).toMatch(/id="nd-calculation-regression-bootstrap-workers"[^>]*min="1"[^>]*max="64"[^>]*value="4"/);
    expect(markup).not.toContain("10,000 resamples are recommended for final results; 1,000 can be used for exploratory runs");
    expect(markup).not.toContain("Percentile intervals are primary");
    expect(markup).not.toContain("BCa is reported when delete-one refits support it");
    expect(markup).not.toContain("studentized intervals, one-tailed tests, and custom alpha are excluded");
    expect(markup).not.toContain("worker-invariant");
    expect(markup).toContain("Start calculation");
    expect(markup).toContain('id="nd-calculation-seed"');

    const standardMarkup = renderReadyDialog(
      "regression",
      {
        ...settings,
        method: "regression",
        preprocessing: "unstandardized",
        regressionType: "ols",
        regressionBootstrap: true,
        bootstrapSamples: 999,
      },
      nodes,
      false,
      undefined,
      undefined,
      capabilityRegistryV2,
    );
    expect(standardMarkup).toContain('id="nd-calculation-regression-bootstrap"');
    expect(standardMarkup).not.toContain('data-method-chip="limited-scope"');
    expect(standardMarkup).not.toContain('data-limited-scope-warning="true"');
    expect(standardMarkup).not.toContain('data-experimental-warning="true"');
  });

  it("stops regression-bootstrap setup at 50 selected predictors and controls", () => {
    const predictors = Array.from({ length: 51 }, (_, index) => `x${index + 1}`);
    const wideDataset: Dataset = {
      id: "wide-bootstrap-data",
      name: "wide-bootstrap.csv",
      columns: ["y", ...predictors],
      rows: [],
      rowCount: 60,
      missing: 0,
      fingerprint: "sha256:wide-bootstrap",
      kind: "raw",
      columnMetadata: ["y", ...predictors].map((name) => metadata(name, "numeric")),
    };
    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "regression",
      experimentalLabsEnabled: true,
      capabilityRegistry: executableLabsRegistry,
      setKind: () => undefined,
      settings: {
        ...settings,
        method: "regression",
        preprocessing: "unstandardized",
        regressionType: "ols",
        regressionOutcome: "y",
        regressionPredictors: predictors.slice(0, 50).join(","),
        regressionBootstrap: true,
        bootstrapSamples: 99,
        workers: 2,
        confidenceLevel: 0.95,
      },
      setSettings: () => undefined,
      readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
      runMonitor,
      dataset: wideDataset,
      analysisColumns: [],
      nodes: [],
      edges: [],
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));

    expect(markup).toContain("Predictors (50 selected)");
    expect(markup).toMatch(/<input type="checkbox" disabled=""\/><span>x51<\/span>/);
  });

  it("renders strict model-free binary logistic setup with a complete 0/1 profile", () => {
    const rows = Array.from({ length: 12 }, (_, index) => ({
      converted: index % 3 === 0 ? 1 : 0,
      score: index + 1,
      age: 22 + index,
      segment: index % 2 ? "B" : "A",
    }));
    const logisticDataset: Dataset = {
      id: "logistic-data",
      name: "logistic.csv",
      columns: ["converted", "score", "age", "segment"],
      rows,
      rowCount: rows.length,
      missing: 0,
      fingerprint: "sha256:logistic",
      kind: "raw",
      columnMetadata: [metadata("converted", "numeric"), metadata("score", "numeric"), metadata("age", "numeric"), metadata("segment", "text")],
    };
    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "regression",
      experimentalLabsEnabled: true,
      capabilityRegistry: executableLabsRegistry,
      setKind: () => undefined,
      settings: {
        ...settings,
        method: "regression",
        preprocessing: "unstandardized",
        workers: 1,
        confidenceLevel: 0.95,
        regressionType: "logistic",
        regressionOutcome: "converted",
        regressionPredictors: "score",
        regressionControls: "age",
        robustSe: "none",
      },
      setSettings: () => undefined,
      readiness: { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] },
      runMonitor,
      dataset: logisticDataset,
      analysisColumns: [],
      nodes: [],
      edges: [],
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));

    expect(markup).toContain('id="nd-calculation-regression-type"');
    expect(markup).toContain('<option value="logistic" selected="">Binary logistic (outcome coded 0/1)</option>');
    expect(markup).toContain('id="nd-calculation-logistic-profile"');
    expect(markup).toContain("Detected values: 0 (8), 1 (4), outside 0/1 (0); 12 complete cases and 0 omitted by listwise deletion");
    expect(markup).not.toContain("Maximum-likelihood SE; Wald z and two-sided 95% CI; odds ratios (fixed)");
    expect(markup).not.toContain("<span>Validated scope</span>");
    expect(markup).not.toContain("The outcome must be coded exactly 0/1");
    expect(markup).toContain("Start calculation");
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');

    const supportedMarkup = renderWithLiveRegistry("regression", {
      ...settings,
      method: "regression",
      regressionType: "logistic",
    });
    expect(supportedMarkup).toMatch(/id="nd-calculation-method-regression"[\s\S]*?<strong>Regression<\/strong><\/span>/);
    expect(supportedMarkup).not.toContain('data-limited-scope-warning="true"');
    expect(supportedMarkup).not.toContain('data-experimental-warning="true"');
  });
});

describe("NativeCalculationDialog unified SEM setup", () => {
  function higherOrderModel(): SemModelV4 {
    const value = convertLegacyBasicModelV4({
      id: "model:dialog-hoc",
      name: "Higher-order model",
      constructs: ["x", "m1", "m2", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: [`${id}1`, `${id}2`],
      })),
      paths: [
        { source: "x", target: "m1" }, { source: "x", target: "m2" },
        { source: "m1", target: "m2" }, { source: "m1", target: "y" },
        { source: "m2", target: "y" }, { source: "x", target: "y" },
      ],
    }, "pls_composite");
    value.variables.push({ kind: "derived", id: "derived:hoc", label: "Organizational strength" });
    value.relations.push({
      kind: "structural",
      id: "relation:hoc_y",
      source: "derived:hoc",
      target: "construct:y",
      parameter: "parameter:hoc_y",
      role: "structural",
      intercept_parameter: null,
    });
    value.parameters.push({
      kind: "free",
      id: "parameter:hoc_y",
      label: "HOC -> Y",
      target: { kind: "regression", source: "derived:hoc", target: "construct:y" },
      group_overrides: [],
    });
    value.derived_terms.push({
      kind: "higher_order",
      id: "term:hoc",
      output: "derived:hoc",
      components: ["construct:m1", "construct:m2"],
      approach: "disjoint_two_stage",
      measurement_type: "reflective_reflective",
    });
    return value;
  }

  function renderUnified(
    kind: "pls_algorithm" | "pls_bootstrap" | "cbsem",
    model: ReturnType<typeof convertLegacyBasicModelV4>,
    methodSettings: AnalysisUiSettings,
    config = defaultGeneralSemConfigV1(),
    legacyReady = true,
  ): string {
    return renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind,
      setKind: () => undefined,
      settings: methodSettings,
      setSettings: () => undefined,
      readiness: legacyReady
        ? { canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] }
        : {
            canRun: false,
            summary: "Legacy route blocked",
            blockers: [{ id: "calculation", label: "Calculation", detail: "Use the removed Exact CB-SEM tab.", status: "blocked" as const }],
            warnings: [],
            items: [],
          },
      runMonitor,
      dataset: { id: "study", name: "study.csv", columns: [], rows: [], missing: 0 },
      analysisColumns: [],
      nodes,
      edges,
      experimentalLabsEnabled: true,
      capabilityRegistry: capabilityRegistryV2,
      unifiedSem: {
        authorityKey: "authority:dialog",
        model,
        config,
      },
      onUnifiedSemAction: () => undefined,
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));
  }

  it("shows detected advanced PLS features without adding another method", () => {
    const mediationModel = convertLegacyBasicModelV4({
      id: "model:dialog-mediation",
      name: "Mediation",
      constructs: ["x", "m1", "m2", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: [`${id}1`, `${id}2`],
      })),
      paths: [
        { source: "x", target: "m1" }, { source: "m1", target: "y" },
        { source: "x", target: "m2" }, { source: "m2", target: "y" },
      ],
    }, "pls_composite");
    const markup = renderUnified("pls_bootstrap", mediationModel, {
      ...settings,
      bootstrapSamples: 500,
      workers: 2,
    });

    expect(markup).toContain('data-unified-sem-calculation="general_sem_pls"');
    expect(markup).toContain("Indirect paths</span><strong>2 detected");
    expect(markup).not.toContain("Detected model features");
    expect(markup).not.toContain("Expected result categories");
    expect(markup).not.toContain("Estimator setup");
    expect(markup.match(/id="nd-calculation-method-[^"]+" type="button" role="option"/g)).toHaveLength(18);
  });

  it("uses one concise researcher-facing row for two- and three-way moderation", () => {
    const common = {
      outputId: "derived:interaction",
      focalRelationId: "relation:x_y",
      predictorId: "construct:x",
      predictorLabel: "Motivation",
      outcomeId: "construct:y",
      outcomeLabel: "Performance",
      parentInteractionTermId: null,
    } as const;

    expect(unifiedInteractionSummaryV1({
      ...common,
      termId: "term:x_w",
      order: "two_way",
      moderatorIds: ["construct:w"],
      moderatorLabels: ["Gender"],
    })).toBe("Gender moderates Motivation → Performance");
    expect(unifiedInteractionSummaryV1({
      ...common,
      termId: "term:x_w_z",
      order: "three_way",
      moderatorIds: ["construct:w", "construct:z"],
      moderatorLabels: ["Ability", "Group"],
      parentInteractionTermId: "term:x_w",
    })).toBe("Group extends Motivation × Ability → Performance");
  });

  it("moves strict CB-SEM inference and the advanced-parameters action into Calculate", () => {
    const factorModel = convertLegacyBasicModelV4({
      id: "model:dialog-cbsem",
      name: "CB-SEM",
      constructs: ["x", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: [`${id}1`, `${id}2`, `${id}3`],
      })),
      paths: [{ source: "x", target: "y" }],
    }, "cbsem_common_factor");
    const config = defaultGeneralSemConfigV1();
    config.inference = {
      kind: "case_bootstrap",
      resamples: 500,
      seed: 7,
      confidence_level: 0.95,
      interval: "percentile",
      tail: "two_sided",
    };
    const markup = renderUnified("cbsem", factorModel, {
      ...settings,
      method: "cbsem",
      cbsemBootstrapSamples: 500,
      workers: 2,
    }, config, false);

    expect(markup).toContain('data-unified-sem-calculation="general_sem_cbsem"');
    expect(markup).not.toContain("common-factor constructs");
    expect(markup).toContain('id="nd-calculation-cbsem-inference"');
    expect(markup).toContain('<option value="case_bootstrap" selected="">Case-resampling bootstrap</option>');
    expect(markup).toContain(">Advanced parameters…</button>");
    expect(markup).toContain('id="nd-calculation-cbsem-bootstrap-samples"');
    expect(markup).toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain('id="nd-calculation-cbsem-model-type"');
    expect(markup).not.toContain("Exact CB-SEM model tab");
    expect(markup).not.toContain("Use the removed Exact CB-SEM tab.");
    expect(markup).toMatch(/class="primary" type="submit">/);
  });

  it("shows one compact HOC row with a calculation-time edit action", () => {
    const markup = renderUnified("pls_algorithm", higherOrderModel(), settings);

    expect(markup).toContain('id="nd-calculation-higher-order"');
    expect(markup).toContain("Organizational strength · RR · disjoint two-stage");
    expect(markup).toContain('aria-label="Edit higher-order construct Organizational strength"');
    expect(markup).toContain(">Edit…</button>");
    expect(markup).not.toContain("<legend>Method settings</legend>");
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup.match(/id="nd-calculation-method-[^"]+" type="button" role="option"/g)).toHaveLength(18);
  });

  it("presents a single-mediation bootstrap through the unified setup without a false blocker", () => {
    const singleMediation = convertLegacyBasicModelV4({
      id: "model:dialog-single-mediation",
      name: "Single mediation",
      constructs: ["x", "m", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: [`${id}1`, `${id}2`],
      })),
      paths: [{ source: "x", target: "m" }, { source: "m", target: "y" }, { source: "x", target: "y" }],
    }, "pls_composite");
    const markup = renderUnified("pls_bootstrap", singleMediation, { ...settings, bootstrapSamples: 500 });

    expect(markup).toContain("Indirect paths</span><strong>1 detected");
    expect(markup).toContain('id="nd-calculation-bootstrap-samples"');
    expect(markup).toContain('id="nd-calculation-confidence"');
    expect(markup).toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain("requires at least two compiled specific indirect paths");
    expect(markup).not.toMatch(/class="nd-blocker"[\s\S]*?<ul>/);
  });
});
