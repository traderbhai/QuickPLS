export type NativePrototypeView = "home" | "data" | "model" | "setup" | "run" | "results" | "report" | "trust" | "settings";
export type NativePrototypeDialog =
  | "new_project"
  | "sample_gallery"
  | "import_data"
  | "calculation_setup"
  | "method_scope"
  | "export_options"
  | "help_shortcuts"
  | "settings"
  | "close_project"
  | "documentation"
  | "data_transform"
  | "data_add_column"
  | "data_recode"
  | "data_missing_values"
  | "data_filter"
  | "data_sort"
  | "release_integrity"
  | null;

export interface NativePrototypeProjectSummary {
  name: string;
  dataset: string;
  cases: number;
  variables: number;
  constructs: number;
  indicators: number;
  paths: number;
  savedRuns: number;
  status: string;
}

export interface NativePrototypeRecentProject {
  name: string;
  path: string;
  modified: string;
  runs: number;
  status: string;
}

export interface NativePrototypeVariable {
  name: string;
  label: string;
  type: string;
  role: string;
  missing: number;
  mean: string;
  sd: string;
}

export interface NativePrototypeConstruct {
  id: string;
  name: string;
  label: string;
  indicators: string[];
  x: number;
  y: number;
  r2?: string;
  indicatorSide?: "left" | "right" | "both" | "top" | "bottom";
  loadings?: string[];
}

export interface NativePrototypeMethodCard {
  lane: string;
  name: string;
  status: string;
  reason: string;
  outputs: string;
}

export interface NativePrototypeDataQuality {
  cases: string;
  variables: string;
  missingValues: string;
  duplicateRows: string;
  constantColumns: string;
  numericVariables: string;
}

export interface NativePrototypeSelectedVariable {
  name: string;
  label: string;
  type: string;
  role: string;
  scale: string;
  missingMarkers: string;
  min: string;
  max: string;
  mean: string;
  sd: string;
  unique: string;
  assignedConstruct: string;
}

export interface NativePrototypeRunSummary {
  state: string;
  currentStep: string;
  progress: number;
  stepProgress: number;
  elapsed: string;
  completedUnits: string;
  totalUnits: string;
  procedure: string[][];
  settings: string[][];
  logs: string[][];
  outputPreviewRows: string[][];
  outputs: string[];
  unavailableOutputs: string[];
}

export interface NativePrototypeResultSummary {
  hasRun: boolean;
  runName: string;
  method: string;
  createdAt: string;
  seed: string;
  fingerprint: string;
  warnings: string;
  strongestR2: string;
  strongestR2Label: string;
  pathCount: string;
  pathRows: string[][];
  r2Rows: string[][];
  reliabilityRows: string[][];
  findings: string[][];
  interpretationTitle: string;
  interpretationBody: string;
  reportWording: string;
}

export interface NativePrototypeReportSummary {
  selectedRun: string;
  destination: string;
  exportReady: boolean;
  hasRun: boolean;
  precision: number;
  palette: string;
  layout: string;
  pathRows: string[][];
  reliabilityRows: string[][];
}

export interface NativePrototypeSettingsSummary {
  precision: string;
  density: string;
  tableDensity: string;
  thresholdColors: boolean;
  defaultExportFormat: string;
}

export interface NativePrototypeData {
  adapterSource: "store" | "fallback";
  projectSummary: NativePrototypeProjectSummary;
  recentProjects: NativePrototypeRecentProject[];
  variables: NativePrototypeVariable[];
  dataHeaders: string[];
  dataRows: string[][];
  dataQuality: NativePrototypeDataQuality;
  selectedVariable: NativePrototypeSelectedVariable;
  methodApplicabilityRows: string[][];
  messages: string[][];
  constructs: NativePrototypeConstruct[];
  paths: string[][];
  methodCards: NativePrototypeMethodCard[];
  resultRows: string[][];
  resultSummary: NativePrototypeResultSummary;
  runSummary: NativePrototypeRunSummary;
  reportSummary: NativePrototypeReportSummary;
  settingsSummary: NativePrototypeSettingsSummary;
  trustRows: string[][];
  selectedMethodLabel: string;
  selectedRunLabel: string;
}

export const projectSummary: NativePrototypeProjectSummary = {
  name: "Customer Loyalty Study",
  dataset: "SurveyData.csv",
  cases: 500,
  variables: 34,
  constructs: 5,
  indicators: 19,
  paths: 6,
  savedRuns: 3,
  status: "Ready",
};

export const recentProjects = [
  { name: "Customer_Loyalty.qpls", path: "D:\\Research\\QuickPLS\\Customer_Loyalty.qpls", modified: "15 May 2025, 10:32", runs: 3, status: "Ready" },
  { name: "Green_Brand_Study.qpls", path: "D:\\Research\\QuickPLS\\Green_Brand_Study.qpls", modified: "12 May 2025, 16:20", runs: 2, status: "Review" },
  { name: "Telehealth_Adoption.qpls", path: "D:\\Research\\QuickPLS\\Telehealth_Adoption.qpls", modified: "09 May 2025, 09:41", runs: 5, status: "Ready" },
  { name: "Sustainability_Readiness.qpls", path: "D:\\Research\\QuickPLS\\Sustainability_Readiness.qpls", modified: "06 May 2025, 11:25", runs: 1, status: "Draft" },
];

export const variables = [
  { name: "AGE", label: "Age", type: "Numeric", role: "Control", missing: 0, mean: "34.1", sd: "8.4" },
  { name: "GENDER", label: "Gender", type: "Nominal", role: "Group", missing: 0, mean: "", sd: "" },
  { name: "INCOME", label: "Income", type: "Numeric", role: "Control", missing: 4, mean: "71200", sd: "12840" },
  { name: "PEOU1", label: "Perceived ease of use 1", type: "Numeric", role: "Indicator", missing: 0, mean: "4.20", sd: "0.91" },
  { name: "PEOU2", label: "Perceived ease of use 2", type: "Numeric", role: "Indicator", missing: 0, mean: "4.40", sd: "0.83" },
  { name: "PU1", label: "Perceived usefulness 1", type: "Numeric", role: "Indicator", missing: 0, mean: "4.60", sd: "0.71" },
  { name: "ATT1", label: "Attitude 1", type: "Numeric", role: "Indicator", missing: 1, mean: "4.30", sd: "0.88" },
  { name: "BI1", label: "Behavioral intention 1", type: "Numeric", role: "Indicator", missing: 0, mean: "4.10", sd: "0.94" },
];

export const dataRows = [
  ["1", "25", "Female", "43000", "4.00", "4.20", "4.50", "4.00", "4.10"],
  ["2", "41", "Male", "72000", "4.20", "4.40", "4.80", "4.30", "4.40"],
  ["3", "38", "Female", "61000", "3.90", "4.10", "4.20", "4.00", "3.80"],
  ["4", "29", "Male", "53000", "4.50", "4.70", "4.90", "4.40", "4.30"],
  ["5", "34", "Female", "69000", "4.10", "4.30", "4.50", "4.20", "4.00"],
  ["6", "47", "Male", "84000", "4.80", "4.70", "5.00", "4.60", "4.70"],
  ["7", "31", "Female", "58000", "3.70", "3.90", "4.10", "3.80", "3.90"],
  ["8", "44", "Male", "76000", "4.30", "4.50", "4.70", "4.20", "4.40"],
];

export const dataHeaders = ["ID", "AGE", "GENDER", "INCOME", "PEOU1", "PEOU2", "PU1", "ATT1", "BI1"];

export const constructs: NativePrototypeConstruct[] = [
  { id: "per", name: "PER", label: "Perceived Quality", indicators: ["PER1", "PER2", "PER3", "PER4"], x: 210, y: 44, indicatorSide: "left", loadings: ["0.821", "0.861", "0.794", "0.742"] },
  { id: "val", name: "VAL", label: "Perceived Value", indicators: ["VAL1", "VAL2", "VAL3", "VAL4"], x: 210, y: 244, r2: "0.298", indicatorSide: "left", loadings: ["0.844", "0.879", "0.802", "0.769"] },
  { id: "sat", name: "SAT", label: "Satisfaction", indicators: ["SAT1", "SAT2", "SAT3", "SAT4"], x: 590, y: 44, r2: "0.318", indicatorSide: "right", loadings: ["0.877", "0.891", "0.846", "0.803"] },
  { id: "tru", name: "TRU", label: "Trust", indicators: ["TRU1", "TRU2", "TRU3"], x: 600, y: 252, r2: "0.467", indicatorSide: "right", loadings: ["0.888", "0.904", "0.842"] },
  { id: "loy", name: "LOY", label: "Loyalty", indicators: ["LOY1", "LOY2", "LOY3", "LOY4", "LOY5", "LOY6"], x: 520, y: 392, r2: "0.589", indicatorSide: "both", loadings: ["0.911", "0.917", "0.866", "0.889", "0.905", "0.836"] },
];

export const paths = [
  ["per", "sat", "0.564"],
  ["per", "val", "0.452"],
  ["per", "tru", "0.315"],
  ["sat", "tru", "0.274"],
  ["val", "loy", "0.310"],
  ["tru", "loy", "0.394"],
];

export const methodCards = [
  { lane: "Recommended", name: "PLS Algorithm", status: "Ready", reason: "Reflective SEM model with prediction focus.", outputs: "Paths, loadings, R-squared, quality criteria" },
  { lane: "Recommended", name: "PLS Bootstrapping", status: "Ready", reason: "Inference-ready path and loading estimates.", outputs: "p values and confidence intervals" },
  { lane: "Recommended", name: "Quality Criteria", status: "Ready", reason: "Measurement and structural checks available.", outputs: "Reliability, validity, VIF, f-squared" },
  { lane: "Available after setup", name: "PLS Predict", status: "Needs target", reason: "Choose prediction target and folds.", outputs: "Q-squared predict, RMSE, MAE" },
  { lane: "Available after setup", name: "Importance Performance Map", status: "Needs target", reason: "Requires a selected endogenous construct.", outputs: "Importance and performance table" },
  { lane: "Available after setup", name: "MGA / MICOM", status: "Needs group", reason: "Requires an observed group column.", outputs: "Invariance and group differences" },
  { lane: "Not applicable", name: "PLSc", status: "Blocked", reason: "Only reflective model variants meet the listed requirements.", outputs: "Consistent loadings and paths" },
  { lane: "Not applicable", name: "Second-order Constructs", status: "No hierarchy", reason: "No hierarchical construct in this model.", outputs: "Higher-order construct estimates" },
  { lane: "Not applicable", name: "Gaussian Copula", status: "Diagnostics", reason: "Requires suitable nonnormal predictors.", outputs: "Endogeneity diagnostic" },
];

export const resultRows = [
  ["PER -> SAT", "0.564", "0.559", "0.071", "7.944", "< 0.001", "Supported"],
  ["PER -> VAL", "0.452", "0.448", "0.068", "6.647", "< 0.001", "Supported"],
  ["PER -> TRU", "0.315", "0.312", "0.074", "4.257", "< 0.001", "Supported"],
  ["SAT -> TRU", "0.274", "0.269", "0.082", "3.341", "0.001", "Supported"],
  ["VAL -> LOY", "0.310", "0.306", "0.077", "4.026", "< 0.001", "Supported"],
  ["TRU -> LOY", "0.394", "0.399", "0.071", "5.549", "< 0.001", "Supported"],
];

export const trustRows = [
  ["Reflective measurement", "Supported", "Supported setup", "Core PLS, PLSc, and CB-SEM requirements"],
  ["Formative measurement", "Supported", "Supported setup", "PLS and GSCA requirements"],
  ["Mediation effects", "Supported", "Supported setup", "Bootstrap recommended for inference"],
  ["CB-SEM CFA", "Limited", "Supported setup", "Raw-data reflective single-group requirements"],
  ["Ordinal WLSMV", "Not supported", "Not available", "Ordinal estimation is not available in this setup"],
  ["SmartPLS project import", "Not supported", "Not available", "Use neutral raw-data or matrix exchange"],
];

export const fallbackNativePrototypeData: NativePrototypeData = {
  adapterSource: "fallback",
  projectSummary,
  recentProjects,
  variables,
  dataHeaders,
  dataRows,
  dataQuality: {
    cases: "500",
    variables: "34",
    missingValues: "2,134 (1.26%)",
    duplicateRows: "0 (0.00%)",
    constantColumns: "0 (0.00%)",
    numericVariables: "28 (82.4%)",
  },
  selectedVariable: {
    name: "BI1",
    label: "Behavioral Intention 1",
    type: "Numeric",
    role: "Indicator",
    scale: "Ordinal",
    missingMarkers: "-, NA",
    min: "1",
    max: "5",
    mean: "3.692",
    sd: "1.071",
    unique: "5",
    assignedConstruct: "Behavioral Intention (BI)",
  },
  methodApplicabilityRows: [
    ["PLS-SEM", "Recommended", "PLS-SEM ready", "Data meets minimum requirements for PLS-SEM."],
    ["CB-SEM (Reflective only)", "Caution", "Check recommended", "Data may be used for CB-SEM (reflective models). Verify multivariate normality."],
    ["Logistic Regression", "Not Applicable", "Needs binary outcome", "No binary (0/1) dependent variable detected."],
    ["NCA", "Not Applicable", "Needs X and Y", "No clear NCA inputs detected. Define one X variable and one Y variable."],
  ],
  messages: [
    ["5/14/2025 9:15:02 AM", "INFO", "Project check", "Project 'Customer_Experience.qpls' passed its readiness checks.", "Customer_Experience.qpls"],
    ["5/14/2025 9:14:58 AM", "INFO", "Data", "Dataset 'CX_Data.csv' loaded. 512 cases, 34 variables.", "Customer_Experience.qpls"],
    ["5/14/2025 9:14:47 AM", "INFO", "Run", "PLS Algorithm completed in 00:00:03.187", "Customer_Experience.qpls"],
  ],
  constructs,
  paths,
  methodCards,
  resultRows,
  resultSummary: {
    hasRun: true,
    runName: "PLS Algorithm",
    method: "PLS-SEM",
    createdAt: "2025-05-18 14:22:31",
    seed: "20240521",
    fingerprint: "a1d72e4c",
    warnings: "2",
    strongestR2: "0.701",
    strongestR2Label: "Use Behavior",
    pathCount: "6",
    pathRows: resultRows,
    r2Rows: [["Use Behavior", "0.701", "0.697", "0.512", "High"]],
    reliabilityRows: [["Quality", "0.872", "0.907", "0.668"]],
    findings: [["Review", "Low Loadings", "3 indicators"], ["Info", "Strong Paths", "5 paths"]],
    interpretationTitle: "Selected Path",
    interpretationBody: "Select a path or result row to review value-specific interpretation.",
    reportWording: "The selected model was estimated with the supported setup and listed requirements.",
  },
  runSummary: {
    state: "Ready",
    currentStep: "Ready to run",
    progress: 0,
    stepProgress: 0,
    elapsed: "00:00:00",
    completedUnits: "0",
    totalUnits: "0",
    procedure: [["1", "Validate Data", "done"], ["2", "Validate Model", "done"], ["3", "Estimate PLS Algorithm", ""], ["4", "Generate Results", ""], ["5", "Save Run", ""]],
    settings: [["Method", "PLS-SEM"], ["Sample Size", "500"], ["Constructs", "5"], ["Paths", "6"], ["Outputs Unavailable", "None"]],
    logs: [["Ready", "INFO", "No run is active."]],
    outputPreviewRows: resultRows,
    outputs: ["Path coefficients", "Loadings", "R-squared", "Reliability"],
    unavailableOutputs: ["None"],
  },
  reportSummary: {
    selectedRun: "PLS Algorithm",
    destination: "C:\\Users\\User\\Documents\\QuickPLS\\Exports\\",
    exportReady: true,
    hasRun: true,
    precision: 3,
    palette: "Grayscale (Print)",
    layout: "Tidy",
    pathRows: resultRows.slice(0, 4),
    reliabilityRows: [["Quality", "0.872", "0.907", "0.668"]],
  },
  settingsSummary: {
    precision: "4",
    density: "Compact",
    tableDensity: "Compact",
    thresholdColors: true,
    defaultExportFormat: "xlsx",
  },
  trustRows,
  selectedMethodLabel: "PLS path modeling",
  selectedRunLabel: "PLS Algorithm",
};
