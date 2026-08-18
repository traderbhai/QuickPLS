import { useMemo } from "react";
import type { Edge, Node } from "@xyflow/react";
import { useWorkspace } from "../store";
import { analysisReadiness } from "../domain/analysisReadiness";
import { dataQualitySummary } from "../domain/dataWorkspace";
import { evaluateMethodApplicability, methodCategoryLabels } from "../domain/methodApplicability";
import { buildResultInterpretation } from "../domain/resultInterpretation";
import { runExportTables } from "../domain/resultTables";
import type { AnalysisMethodId, AnalysisRun, AnalysisUiSettings, ConstructData, Dataset, RunMonitorState, UiPreferences } from "../types";
import {
  fallbackNativePrototypeData,
  type NativePrototypeConstruct,
  type NativePrototypeData,
  type NativePrototypeDataQuality,
  type NativePrototypeMethodCard,
  type NativePrototypeRecentProject,
  type NativePrototypeResultSummary,
  type NativePrototypeRunSummary,
  type NativePrototypeSelectedVariable,
  type NativePrototypeSettingsSummary,
  type NativePrototypeVariable,
} from "./nativePrototypeData";

const methodLabels: Record<AnalysisMethodId, string> = {
  pls_pm: "PLS path modeling",
  pls_sample_size_power: "PLS sample-size and power analysis",
  bootstrap: "PLS bootstrapping",
  permutation: "Structural Path Randomization",
  plsc: "Consistent PLS",
  wpls: "Weighted PLS",
  cca: "Confirmatory composite analysis",
  cta_pls: "Confirmatory tetrad analysis",
  endogeneity: "Gaussian copula endogeneity",
  nonlinear_effects: "Nonlinear effects",
  moderated_mediation: "Moderated mediation",
  predict: "PLSpredict",
  mga: "MICOM / MGA",
  ipma: "IPMA",
  cbsem: "CB-SEM / CFA",
  pca: "PCA",
  gsca: "GSCA",
  regression: "Regression",
  nca: "NCA",
};

const numberText = (value: number | null | undefined, decimals = 3) =>
  Number.isFinite(value) ? String(Number(value).toFixed(decimals)) : "";

const valueText = (value: string | number | null | undefined) => {
  if (value === null || value === undefined) return "";
  return String(value);
};

const emptyDataQuality: NativePrototypeDataQuality = {
  cases: "0",
  variables: "0",
  missingValues: "0 (0.00%)",
  duplicateRows: "0 (0.00%)",
  constantColumns: "0 (0.00%)",
  numericVariables: "0 (0.0%)",
};

const noSelectedVariable: NativePrototypeSelectedVariable = {
  name: "No variable selected",
  label: "Select a variable",
  type: "-",
  role: "-",
  scale: "-",
  missingMarkers: "-",
  min: "-",
  max: "-",
  mean: "-",
  sd: "-",
  unique: "0",
  assignedConstruct: "Unassigned",
};

const noRunResultSummary: NativePrototypeResultSummary = {
  hasRun: false,
  runName: "No completed run selected",
  method: "No run",
  createdAt: "",
  seed: "-",
  fingerprint: "",
  warnings: "0",
  strongestR2: "N/A",
  strongestR2Label: "No completed run",
  pathCount: "0",
  pathRows: [],
  r2Rows: [],
  reliabilityRows: [],
  findings: [["info", "No completed run", "Run an analysis to populate result tables."]],
  interpretationTitle: "No completed run",
  interpretationBody: "Run a compatible method to show value-specific interpretation.",
  reportWording: "No completed run is selected.",
};

const RECENT_PROJECTS_KEY = "quickpls.native.recentProjects.v1";

const readRecentProjects = (): NativePrototypeRecentProject[] => {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(RECENT_PROJECTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((item): item is NativePrototypeRecentProject =>
        item && typeof item.name === "string" && typeof item.path === "string",
      )
      .map((item) => ({
        name: item.name,
        path: item.path,
        modified: item.modified || "Previously opened",
        runs: Number.isFinite(Number(item.runs)) ? Number(item.runs) : 0,
        status: item.status || "Unknown",
      }))
      .slice(0, 12);
  } catch {
    return [];
  }
};

const writeRecentProjects = (projects: NativePrototypeRecentProject[]) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(projects.slice(0, 12)));
  } catch {
    // Recent projects are convenience-only; failing to persist must not affect the project.
  }
};

const numericValuesForColumn = (dataset: Dataset, column: string) =>
  dataset.rows
    .map((row) => row[column])
    .map((value) => typeof value === "number" ? value : Number(value))
    .filter((value) => Number.isFinite(value));

const mean = (values: number[]) => values.length ? values.reduce((sum, value) => sum + value, 0) / values.length : null;

const sampleSd = (values: number[]) => {
  if (values.length < 2) return null;
  const avg = mean(values);
  if (avg === null) return null;
  return Math.sqrt(values.reduce((sum, value) => sum + ((value - avg) ** 2), 0) / (values.length - 1));
};

const indicatorOwner = (column: string, nodes: Array<Node<ConstructData>>) =>
  nodes.find((node) => node.data.indicators.includes(column));

const adaptVariables = (dataset: Dataset, nodes: Array<Node<ConstructData>>): NativePrototypeVariable[] => {
  const metadataByName = new Map((dataset.columnMetadata ?? []).map((item) => [item.name, item]));
  return dataset.columns.slice(0, 24).map((column) => {
    const values = numericValuesForColumn(dataset, column);
    const metadata = metadataByName.get(column);
    const owner = indicatorOwner(column, nodes);
    const missing = dataset.rows.filter((row) => row[column] === null || row[column] === "" || row[column] === undefined).length;
    return {
      name: column,
      label: metadata?.label || column,
      type: metadata?.column_type === "text" ? "Text" : metadata?.column_type === "boolean" ? "Boolean" : "Numeric",
      role: owner ? "Indicator" : metadata?.scale_type === "binary" || metadata?.scale_type === "nominal" ? "Group" : "Unassigned",
      missing,
      mean: numberText(mean(values)),
      sd: numberText(sampleSd(values)),
    };
  });
};

const adaptRows = (dataset: Dataset) => {
  const columns = dataset.columns.slice(0, 16);
  const rows = dataset.rows.slice(0, 12).map((row, index) => [String(index + 1), ...columns.map((column) => valueText(row[column]))]);
  return { headers: ["ID", ...columns], rows };
};

const normalizedConstructPositions = (nodes: Array<Node<ConstructData>>): NativePrototypeConstruct[] => {
  const xs = nodes.map((node) => node.position.x);
  const ys = nodes.map((node) => node.position.y);
  const minX = Math.min(...xs);
  const minY = Math.min(...ys);
  const maxX = Math.max(...xs);
  const maxY = Math.max(...ys);
  const width = Math.max(1, maxX - minX);
  const height = Math.max(1, maxY - minY);
  return nodes.map((node) => ({
    id: node.id,
    name: node.data.shortName || node.data.label,
    label: node.data.label,
    indicators: [...node.data.indicators],
    x: Math.round(145 + ((node.position.x - minX) / width) * 705),
    y: Math.round(112 + ((node.position.y - minY) / height) * 226),
  }));
};

const pathCoefficientMap = (run: AnalysisRun | undefined) => {
  const map = new Map<string, string>();
  for (const path of run?.result?.paths ?? []) {
    map.set(`${path.source}->${path.target}`, path.coefficient.toFixed(3));
  }
  return map;
};

const adaptPaths = (edges: Edge[], latestRun: AnalysisRun | undefined) => {
  const coefficients = pathCoefficientMap(latestRun);
  return edges
    .filter((edge) => edge.source && edge.target && edge.data?.role !== "covariance")
    .slice(0, 30)
    .map((edge) => [edge.source, edge.target, coefficients.get(`${edge.source}->${edge.target}`) ?? String(edge.label ?? "Path")]);
};

const adaptMethodCards = (dataset: Dataset, nodes: Array<Node<ConstructData>>, edges: Edge[], settings: AnalysisUiSettings): NativePrototypeMethodCard[] => {
  const cards = evaluateMethodApplicability({ dataset, nodes, edges, settings, nativeDesktop: true });
  return cards.map((card) => ({
    lane: card.status === "recommended" ? "Recommended" : card.status === "available" || card.status === "needs_setup" ? "Available after setup" : "Not applicable",
    name: card.method.name,
    status: card.status === "recommended" || card.status === "available" ? "Ready" : card.status === "needs_setup" ? "Needs setup" : card.status === "experimental" ? "Experimental" : "Blocked",
    reason: card.reason,
    outputs: `${methodCategoryLabels[card.category]}: ${card.expectedOutputs.join(", ")}`,
  })).slice(0, 18);
};

const adaptResultRows = (latestRun: AnalysisRun | undefined) => {
  const paths = latestRun?.result?.paths ?? [];
  if (!paths.length) return [];
  return paths.map((path) => [
    `${path.source} -> ${path.target}`,
    path.coefficient.toFixed(3),
    path.coefficient.toFixed(3),
    "N/A",
    "N/A",
    "N/A",
    latestRun?.bootstrap ? "Inference available" : "Estimate only",
  ]);
};

const adaptDataQuality = (dataset: Dataset): NativePrototypeDataQuality => {
  if (!dataset.columns.length) return emptyDataQuality;
  const summary = dataQualitySummary(dataset);
  const totalCells = Math.max(1, dataset.rows.length * dataset.columns.length);
  return {
    cases: String(dataset.sampleSize ?? dataset.rowCount ?? dataset.rows.length),
    variables: String(dataset.columns.length),
    missingValues: `${summary.missingCells} (${((summary.missingCells / totalCells) * 100).toFixed(2)}%)`,
    duplicateRows: "0 (0.00%)",
    constantColumns: `${summary.constantColumns.length} (${((summary.constantColumns.length / Math.max(1, dataset.columns.length)) * 100).toFixed(2)}%)`,
    numericVariables: `${summary.numericVariables} (${((summary.numericVariables / Math.max(1, dataset.columns.length)) * 100).toFixed(1)}%)`,
  };
};

const adaptSelectedVariable = (dataset: Dataset, nodes: Array<Node<ConstructData>>): NativePrototypeSelectedVariable => {
  const column = dataset.columns[0];
  if (!column) return noSelectedVariable;
  const values = numericValuesForColumn(dataset, column);
  const unique = new Set(dataset.rows.map((row) => valueText(row[column]))).size;
  const owner = indicatorOwner(column, nodes);
  const metadata = dataset.columnMetadata?.find((item) => item.name === column);
  return {
    name: column,
    label: metadata?.label || column,
    type: metadata?.column_type === "text" ? "Text" : metadata?.column_type === "boolean" ? "Boolean" : "Numeric",
    role: owner ? "Indicator" : "Unassigned",
    scale: metadata?.scale_type || "continuous",
    missingMarkers: ", NA, N/A, .",
    min: numberText(values.length ? Math.min(...values) : null, 3) || "-",
    max: numberText(values.length ? Math.max(...values) : null, 3) || "-",
    mean: numberText(mean(values), 3) || "-",
    sd: numberText(sampleSd(values), 3) || "-",
    unique: String(unique),
    assignedConstruct: owner ? `${owner.data.label} (${owner.data.shortName || owner.id})` : "Unassigned",
  };
};

const methodApplicabilityRows = (dataset: Dataset, nodes: Array<Node<ConstructData>>, edges: Edge[], settings: AnalysisUiSettings) =>
  evaluateMethodApplicability({ dataset, nodes, edges, settings, nativeDesktop: true })
    .slice(0, 8)
    .map((item) => [
      item.method.name,
      item.status === "recommended" ? "Recommended" : item.status === "available" ? "Available" : item.status === "needs_setup" ? "Needs setup" : item.status === "experimental" ? "Experimental" : "Not Applicable",
      item.status === "recommended" || item.status === "available" ? "Ready" : item.checks.find((req) => req.status !== "passed")?.label ?? item.status,
      item.reason,
    ]);

const formatTime = (iso: string | null | undefined) => {
  if (!iso) return "";
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleTimeString();
};

const elapsedText = (start: string | null, end: string | null) => {
  if (!start) return "00:00:00";
  const startMs = new Date(start).getTime();
  const endMs = end ? new Date(end).getTime() : Date.now();
  if (!Number.isFinite(startMs) || !Number.isFinite(endMs)) return "00:00:00";
  const total = Math.max(0, Math.round((endMs - startMs) / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  return [hours, minutes, seconds].map((part) => String(part).padStart(2, "0")).join(":");
};

const adaptRunSummary = (runMonitor: RunMonitorState, latestRun: AnalysisRun | undefined, dataset: Dataset, nodes: Array<Node<ConstructData>>, edges: Edge[], settings: AnalysisUiSettings): NativePrototypeRunSummary => {
  const progress = runMonitor.totalUnits > 0 ? Math.round((runMonitor.completedUnits / runMonitor.totalUnits) * 100) : latestRun?.status === "completed" ? 100 : 0;
  const logs = runMonitor.logs.length
    ? runMonitor.logs.slice(0, 12).map((log) => [formatTime(log.timestamp), log.tone.toUpperCase(), `${log.phase}: ${log.message}`])
    : latestRun
      ? [[formatTime(latestRun.createdAt), latestRun.status === "completed" ? "INFO" : "WARN", `${latestRun.name} ${latestRun.status}.`]]
      : [["Ready", "INFO", "No active run. Start a calculation to monitor progress here."]];
  return {
    state: runMonitor.status,
    currentStep: runMonitor.phase || (latestRun?.status === "completed" ? "Completed" : "Ready"),
    progress,
    stepProgress: progress,
    elapsed: elapsedText(runMonitor.startedAt, runMonitor.completedAt),
    completedUnits: String(runMonitor.completedUnits || (latestRun?.status === "completed" ? 1 : 0)),
    totalUnits: String(runMonitor.totalUnits || (latestRun?.status === "completed" ? 1 : 0)),
    procedure: [
      ["1", "Validate Data", dataset.columns.length ? "done" : ""],
      ["2", "Validate Model", nodes.length ? "done" : ""],
      ["3", "Estimate PLS Algorithm", latestRun?.result ? "done" : runMonitor.status === "running" ? "active" : ""],
      ["4", settings.bootstrapSamples > 0 ? "Bootstrap" : "Generate Results", latestRun?.bootstrap ? "done" : runMonitor.status === "running" ? "active" : ""],
      ["5", "Save Run", latestRun?.status === "completed" ? "done" : ""],
    ],
    settings: [
      ["Method", methodLabels[settings.method] ?? settings.method],
      ["Sample Size", String(dataset.sampleSize ?? dataset.rowCount ?? dataset.rows.length)],
      ["Constructs", String(nodes.length)],
      ["Indicators", String(nodes.reduce((sum, node) => sum + node.data.indicators.length, 0))],
      ["Paths", String(edges.filter((edge) => edge.data?.role !== "covariance").length)],
      ["Bootstrap Samples", String(settings.bootstrapSamples || 0)],
      ["Seed", String(settings.seed)],
      ["Workers", String(settings.workers)],
      ["Data Fingerprint", latestRun?.fingerprint.slice(0, 12) ?? "Not run"],
      ["Outputs to be Produced", "Path coefficients, loadings, R-squared, reliability, validity, exports"],
      ["Outputs Unavailable", settings.bootstrapSamples > 0 ? "None requested as unavailable" : "Bootstrap inference unless enabled"],
    ],
    logs,
    outputPreviewRows: adaptResultRows(latestRun),
    outputs: ["Path coefficients", "Loadings / weights", "R-squared", "Reliability", "Validity", "Exports"],
    unavailableOutputs: settings.bootstrapSamples > 0 ? ["None"] : ["Bootstrap p values and confidence intervals"],
  };
};

const adaptResultSummary = (latestRun: AnalysisRun | undefined, nodes: Array<Node<ConstructData>>, edges: Edge[]): NativePrototypeResultSummary => {
  if (!latestRun?.result) return noRunResultSummary;
  const result = latestRun.result;
  const pathRows = adaptResultRows(latestRun);
  const r2Entries = Object.entries(result.r_squared).sort((a, b) => b[1] - a[1]);
  const strongest = r2Entries[0];
  const interpretation = buildResultInterpretation({ run: latestRun, nodes, edges });
  const findings = interpretation.findings.slice(0, 8).map((finding) => [finding.severity, finding.metric, finding.value]);
  return {
    hasRun: true,
    runName: latestRun.name,
    method: methodLabels[(latestRun.method as AnalysisMethodId)] ?? latestRun.method,
    createdAt: latestRun.createdAt,
    seed: String(latestRun.seed),
    fingerprint: latestRun.fingerprint,
    warnings: String((latestRun.warnings?.length ?? 0) + (result.warnings?.length ?? 0)),
    strongestR2: strongest ? strongest[1].toFixed(4) : "N/A",
    strongestR2Label: strongest?.[0] ?? "No endogenous construct",
    pathCount: String(result.paths.length),
    pathRows,
    r2Rows: (latestRun.assessment?.structural_quality?.length
      ? latestRun.assessment.structural_quality.map((row) => [row.construct, row.r_squared.toFixed(4), row.adjusted_r_squared == null ? "N/A" : row.adjusted_r_squared.toFixed(4), "N/A", row.r_squared >= 0.5 ? "substantial" : row.r_squared >= 0.25 ? "moderate" : "limited"])
      : r2Entries.map(([construct, value]) => [construct, value.toFixed(4), "N/A", "N/A", value >= 0.5 ? "substantial" : value >= 0.25 ? "moderate" : "limited"])),
    reliabilityRows: latestRun.assessment?.construct_quality?.map((row) => [
      row.construct,
      row.cronbach_alpha == null ? "N/A" : row.cronbach_alpha.toFixed(4),
      row.rho_c == null ? "N/A" : row.rho_c.toFixed(4),
      row.ave == null ? "N/A" : row.ave.toFixed(4),
    ]) ?? [],
    findings,
    interpretationTitle: interpretation.findings[0]?.metric ?? "Run interpretation",
    interpretationBody: interpretation.findings[0]?.interpretation ?? "Select a row to view value-specific interpretation.",
    reportWording: interpretation.reportParagraphs[0]?.text ?? "The selected run was reviewed against its requirements and known limitations.",
  };
};

const adaptSettingsSummary = (uiPreferences: UiPreferences): NativePrototypeSettingsSummary => ({
  precision: String(uiPreferences.defaultPrecision),
  density: uiPreferences.density,
  tableDensity: uiPreferences.tableDensity,
  thresholdColors: uiPreferences.showThresholdColors,
  defaultExportFormat: uiPreferences.selectedExportPreset,
});

const adaptTrustRows = (dataset: Dataset, nodes: Array<Node<ConstructData>>, edges: Edge[], settings: AnalysisUiSettings) =>
  evaluateMethodApplicability({ dataset, nodes, edges, settings, nativeDesktop: true })
    .slice(0, 12)
    .map((item) => [
      item.method.name,
      methodCategoryLabels[item.category],
      item.status === "recommended" || item.status === "available" ? "Supported setup" : item.status,
      item.reason,
    ]);

export function useNativePrototypeAdapter(): NativePrototypeData {
  const mockupParity = typeof window !== "undefined" && new URLSearchParams(window.location.search).get("mockup_parity") === "1";
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const projectName = useWorkspace((state) => state.projectName);
  const projectPath = useWorkspace((state) => state.projectPath);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const runMonitor = useWorkspace((state) => state.runMonitor);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);

  return useMemo(() => {
    if (mockupParity) {
      return fallbackNativePrototypeData;
    }
    const completedRuns = runs.filter((run) => run.status === "completed" && run.result);
    const selectedCompletedRun = completedRuns.find((run) => run.id === selectedResultRunId);
    const latestCompletedRun = selectedCompletedRun ?? completedRuns[0];
    const runMonitorRun = runs.find((run) => run.id === selectedResultRunId) ?? latestCompletedRun ?? runs[0];
    const adaptedRows = adaptRows(dataset);
    const adaptedConstructs = nodes.length ? normalizedConstructPositions(nodes) : [];
    const adaptedPaths = edges.length ? adaptPaths(edges, latestCompletedRun) : [];
    const readiness = analysisReadiness({ dataset, nodes, edges, settings: analysisSettings, nativeDesktop: true });
    const resultSummary = adaptResultSummary(latestCompletedRun, nodes, edges);
    const exportTables = latestCompletedRun ? runExportTables(latestCompletedRun) : [];
    const projectSummary = {
      name: projectPath ? projectPath.split(/[\\/]/).pop() || projectName : projectName || "No project open",
      dataset: dataset.name || "No dataset loaded",
      cases: dataset.sampleSize ?? dataset.rowCount ?? dataset.rows.length,
      variables: dataset.columns.length,
      constructs: nodes.length,
      indicators: nodes.reduce((sum, node) => sum + node.data.indicators.length, 0),
      paths: edges.filter((edge) => edge.data?.role !== "covariance").length,
      savedRuns: runs.length,
      status: readiness.canRun ? "Ready" : runMonitorRun?.status === "failed" ? "Review" : "Needs setup",
    };
    const currentRecentProject = projectPath
      ? { name: projectSummary.name, path: projectPath, modified: "Current session", runs: runs.length, status: projectSummary.status }
      : null;
    const storedRecentProjects = readRecentProjects();
    const recentProjects = currentRecentProject
      ? [currentRecentProject, ...storedRecentProjects.filter((project) => project.path !== projectPath)].slice(0, 12)
      : storedRecentProjects;
    if (currentRecentProject) {
      writeRecentProjects(recentProjects);
    }
    return {
      adapterSource: "store",
      projectSummary,
      recentProjects,
      variables: adaptVariables(dataset, nodes),
      dataHeaders: adaptedRows.headers,
      dataRows: adaptedRows.rows,
      dataQuality: adaptDataQuality(dataset),
      selectedVariable: adaptSelectedVariable(dataset, nodes),
      methodApplicabilityRows: methodApplicabilityRows(dataset, nodes, edges, analysisSettings),
      messages: [
        [new Date().toLocaleString(), readiness.canRun ? "INFO" : "WARNING", "Readiness", readiness.summary, projectSummary.name],
        [new Date().toLocaleString(), "INFO", "Data", dataset.columns.length ? `Dataset '${projectSummary.dataset}' loaded. ${projectSummary.cases} cases, ${projectSummary.variables} variables.` : "No dataset loaded.", projectSummary.name],
        [new Date().toLocaleString(), "INFO", "Model", `${projectSummary.constructs} constructs, ${projectSummary.indicators} indicators, ${projectSummary.paths} structural paths.`, projectSummary.name],
        ...(runMonitorRun ? [[new Date(runMonitorRun.createdAt).toLocaleString(), runMonitorRun.status === "completed" ? "INFO" : "WARNING", "Run", `${runMonitorRun.name} ${runMonitorRun.status}.`, projectSummary.name]] : []),
      ],
      constructs: adaptedConstructs,
      paths: adaptedPaths,
      methodCards: adaptMethodCards(dataset, nodes, edges, analysisSettings),
      resultRows: adaptResultRows(latestCompletedRun),
      resultSummary,
      runSummary: adaptRunSummary(runMonitor, runMonitorRun, dataset, nodes, edges, analysisSettings),
      reportSummary: {
        selectedRun: latestCompletedRun?.name ?? "No completed run selected",
        destination: "Use the export dialog to choose a destination",
        exportReady: Boolean(latestCompletedRun?.result),
        hasRun: Boolean(latestCompletedRun?.result),
        precision: publicationDiagramSettings.precision ?? uiPreferences.defaultPrecision,
        palette: publicationDiagramSettings.palette,
        layout: publicationDiagramSettings.layoutSource,
        pathRows: resultSummary.pathRows.slice(0, 5),
        reliabilityRows: resultSummary.reliabilityRows.length ? resultSummary.reliabilityRows : exportTables.find((table) => /reliability/i.test(table.title))?.rows.slice(0, 6) ?? [],
      },
      settingsSummary: adaptSettingsSummary(uiPreferences),
      trustRows: adaptTrustRows(dataset, nodes, edges, analysisSettings),
      selectedMethodLabel: methodLabels[analysisSettings.method] ?? analysisSettings.method,
      selectedRunLabel: latestCompletedRun?.name ?? "No completed run selected",
    };
  }, [analysisSettings, dataset, edges, mockupParity, nodes, projectName, projectPath, publicationDiagramSettings, runMonitor, runs, selectedResultRunId, uiPreferences]);
}
