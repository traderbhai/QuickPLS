import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset, RunMonitorState } from "../types";
import {
  default as NativeCalculationDialog,
  NATIVE_RESAMPLING_SAMPLE_INPUT_CONSTRAINTS,
  nativeRegressionTypeSettingsPatch,
  nativeNumericCaseWeightColumns,
  retryNativeProcessProfileState,
  shouldStartNativeProcessProfile,
  scrollNativeMethodOptionIntoView,
} from "./NativeCalculationDialog";
import type { NativeWorkbenchAnalysisKind } from "./nativeAnalysisCatalog";

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

function renderReadyDialog(
  kind: NativeWorkbenchAnalysisKind,
  methodSettings: AnalysisUiSettings,
  analysisNodes: Array<Node<ConstructData>> = nodes,
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
    start: () => undefined,
    cancel: () => undefined,
    close: () => undefined,
  }));
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

describe("NativeCalculationDialog contracts", () => {
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

  it("renders formerly step-mismatched valid counts with an enabled submit action", () => {
    const bootstrap = renderReadyDialog("pls_bootstrap", { ...settings, bootstrapSamples: 999 });
    expect(bootstrap).toMatch(/id="nd-calculation-bootstrap-samples"[^>]*step="1"[^>]*value="999"/);
    expect(bootstrap).toMatch(/class="primary" type="submit"/);

    const permutation = renderReadyDialog("pls_permutation", { ...settings, permutationSamples: 4_321 });
    expect(permutation).toMatch(/id="nd-calculation-permutations"[^>]*step="1"[^>]*value="4321"/);
    expect(permutation).toContain("Candidate scope");
    expect(permutation).toContain("fixed original PLS construct scores");
    expect(permutation).toContain("no multiplicity adjustment");
    expect(permutation).toMatch(/class="primary" type="submit"/);
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

  it("renders the bounded joint MICOM and two-group permutation scope", () => {
    const markup = renderReadyDialog("mga", {
      ...settings,
      method: "mga",
      groupColumn: "group",
      groupAValue: "A",
      groupBValue: "B",
      groupMethods: "micom,mga_permutation",
      groupPermutationSamples: 5_000,
      micomConfiguralConfirmed: false,
    });

    expect(markup).toContain("MICOM and Two-Group Permutation MGA");
    expect(markup).toContain('id="nd-calculation-group-column"');
    expect(markup).toMatch(/id="nd-calculation-group-permutations"[^>]*min="5000"[^>]*max="10000"[^>]*step="1"[^>]*value="5000"/);
    expect(markup).toContain('id="nd-calculation-micom-confidence"');
    expect(markup).toContain('id="nd-calculation-micom-configural"');
    expect(markup).toContain("Two-tailed; Group A");
    expect(markup).toContain("Confirm MICOM Step 1");
    expect(markup).toContain("Step 2 composition and Step 3 pooled-score means and variances");
    expect(markup).not.toContain("Parallel workers");
    expect(markup).toContain("configural invariance");
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
    expect(markup).toContain('aria-describedby="nd-calculation-method-cca-description"');
    expect(markup).toContain('id="nd-calculation-panel-cca-title"');
    expect(markup).toContain("CCA composite residual diagnostics");
    expect(markup).toContain("Standardized (fixed)");
    expect(markup).toContain("Reflective composite path model; descriptive residual diagnostics only");
    expect(markup).toContain("Listwise deletion");
    expect(markup).toContain("Start composite diagnostics");
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
    const markup = renderReadyDialog("cta_pls", {
      ...settings,
      method: "cta_pls",
      weightingScheme: "path",
      preprocessing: "standardized",
    }, ctaNodes);
    expect(markup).toMatch(/id="nd-calculation-method-cta_pls"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain('id="nd-calculation-cta-pls-scope"');
    expect(markup).toContain("Capability: 4 indicators, 3 tetrads");
    expect(markup).toContain("Descriptive sample-covariance tetrads only");
    expect(markup).toContain("Start tetrad diagnostics");
    expect(markup).not.toContain('id="nd-calculation-bootstrap-samples"');
    expect(markup).not.toContain('id="nd-calculation-permutations"');
  });

  it("renders one ID-backed endogenous IPMA target with fixed truthful settings", () => {
    const markup = renderReadyDialog("ipma", {
      ...settings,
      method: "ipma",
      weightingScheme: "path",
      preprocessing: "standardized",
      ipmaTargets: "y",
    });

    expect(markup).toMatch(/id="nd-calculation-method-ipma"[^>]*role="option"[^>]*aria-selected="true"/);
    expect(markup).toContain("Importance-Performance Map Analysis");
    expect(markup).toContain('id="nd-calculation-ipma-target"');
    expect(markup).toMatch(/id="nd-calculation-ipma-target"[^>]*required=""/);
    expect(markup).toContain('<option value="">Select one endogenous construct</option><option value="y" selected="">Retention [y]</option>');
    expect(markup).toContain("Path weighting (fixed)");
    expect(markup).toContain("Standardized (fixed)");
    expect(markup).toContain("Listwise deletion");
    expect(markup).toContain("Direct and indirect structural predecessors only; the target and unrelated constructs are omitted");
    expect(markup).toContain("0-100 observed-range scaling of standardized composite scores; no theoretical-range correction");
    expect(markup).toContain("Start importance-performance analysis");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-confidence"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain("Case-weight variable");
  });

  it("renders the fixed indicator-level PLSpredict / CVPAT contract without irrelevant controls", () => {
    const markup = renderReadyDialog("predict", { ...settings, method: "predict" });

    expect(markup).toMatch(/id="nd-calculation-method-predict"[^>]*aria-selected="true"/);
    expect(markup).toContain("PLSpredict / CVPAT");
    expect(markup).toContain('id="nd-calculation-prediction-plan"');
    expect(markup).toContain("10-fold × 10-repeat");
    expect(markup).toContain("Endogenous indicators are primary");
    expect(markup).toContain("Indicator average (IA) and Linear model (LM, where estimable)");
    expect(markup).toContain("one-sided test, 95% confidence; not a comparison of saved models");
    expect(markup).toContain("Start prediction");
    expect(markup).toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-confidence"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
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
    expect(markup).toContain("Multiple conditions, latent-score NCA, cIPMA");
    expect(markup).toContain("Start necessary condition analysis");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain("Endogenous target");
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
    expect(markup).toContain('id="nd-calculation-cbsem-estimator"');
    expect(markup).toContain("Maximum likelihood; first loading fixed to 1 for each latent factor");
    expect(markup).toContain('id="nd-calculation-cbsem-scope"');
    expect(markup).toContain("Single-group reflective raw-data CFA or recursive SEM");
    expect(markup).toContain("Start CB-SEM / CFA");
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
    expect(markup).toContain('id="nd-calculation-gsca-estimator"');
    expect(markup).toContain('id="nd-calculation-gsca-scope"');
    expect(markup).toContain("Joint global least-squares alternating least squares");
    expect(markup).toContain("Start GSCA");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
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
    expect(markup).toContain("Correlation matrix (fixed)");
    expect(markup).toContain("Standardized numeric values (fixed)");
    expect(markup).toContain("deterministic component orientation");
    expect(markup).toContain("Start principal component analysis");
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
    expect(markup).toContain("HC3 robust SE; two-sided 95% CI (fixed)");
    expect(markup).toContain("Raw numeric ordinary least squares with an intercept");
    expect(markup).toContain("Start OLS regression");
    expect(markup).not.toContain('id="nd-calculation-weighting"');
    expect(markup).not.toContain('id="nd-calculation-preprocessing"');
    expect(markup).not.toContain('id="nd-calculation-max-iterations"');
    expect(markup).not.toContain('id="nd-calculation-tolerance"');
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
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
    expect(markup).toContain("10,000 resamples are recommended for final results; 1,000 can be used for exploratory runs");
    expect(markup).toContain("Percentile intervals are primary");
    expect(markup).toContain("BCa is reported when delete-one refits support it");
    expect(markup).toContain("studentized intervals, one-tailed tests, and custom alpha are excluded");
    expect(markup).toContain("worker-invariant");
    expect(markup).toContain("Start OLS regression with bootstrap");
    expect(markup).toContain('id="nd-calculation-seed"');
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
    expect(markup).toContain("12 complete cases: 8 class 0 and 4 class 1; 0 omitted by listwise deletion");
    expect(markup).toContain("Maximum-likelihood SE; Wald z and two-sided 95% CI; odds ratios (fixed)");
    expect(markup).toContain("The outcome must be coded exactly 0/1");
    expect(markup).toContain("Start binary logistic regression");
    expect(markup).not.toContain('id="nd-calculation-seed"');
    expect(markup).not.toContain('id="nd-calculation-workers"');
  });
});
