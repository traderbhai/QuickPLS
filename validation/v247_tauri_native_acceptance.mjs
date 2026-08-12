import fs from "node:fs/promises";
import path from "node:path";
import { execFile, spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { chromium } from "playwright";

const execFileAsync = promisify(execFile);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const screenshotDir = path.join(root, "validation", "results", "screens", "v247-native-desktop-acceptance");
const reportPath = path.join(root, "validation", "results", "v247_tauri_native_acceptance.json");
const validationResultsDir = path.join(root, "validation", "results");
const windowsNativeSaveHelperPath = path.join(root, "validation", "windows_native_save_export.py");
const endpoint = process.env.QUICKPLS_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const acceptanceScope = process.env.QUICKPLS_ACCEPTANCE_SCOPE?.trim().toLocaleLowerCase() || "full";
if (!["full", "mga", "nca", "prediction", "hoc", "pca", "ols", "cbsem", "gsca"].includes(acceptanceScope)) {
  throw new Error(`QUICKPLS_ACCEPTANCE_SCOPE must be "full", "mga", "nca", "prediction", "hoc", "pca", "ols", "cbsem", or "gsca"; received ${acceptanceScope}.`);
}
const ncaOnly = acceptanceScope === "nca";
const mgaOnly = acceptanceScope === "mga";
const predictionOnly = acceptanceScope === "prediction";
const hocOnly = acceptanceScope === "hoc";
const pcaOnly = acceptanceScope === "pca";
const olsOnly = acceptanceScope === "ols";
const cbsemOnly = acceptanceScope === "cbsem";
const gscaOnly = acceptanceScope === "gsca";
const focusedOnly = ncaOnly || mgaOnly || predictionOnly || hocOnly || pcaOnly || olsOnly || cbsemOnly || gscaOnly;
const scopedReportPath = focusedOnly
  ? path.join(root, "validation", "results", `v247_tauri_native_acceptance_${acceptanceScope}.json`)
  : reportPath;
const recentProjectsKey = "quickpls.native.recent-projects.v1";
const fixtureCsvPath = path.join(root, "validation", "results", "wpls_reference.csv");
const mediationFixtureCsvPath = path.join(root, "validation", "results", "lavaan_latent_mediation_sem.csv");
const moderationFixtureCsvPath = path.join(root, "validation", "results", "moderation_reference_base.csv");
const hocFixtureCsvPath = path.join(root, "validation", "results", "higher_order_two_stage_base.csv");
const mgaFixtureCsvPath = path.join(root, "validation", "results", "mga_reference.csv");
const ccaFixtureCsvPath = path.join(root, "validation", "results", "cca_reference.csv");
const ipmaFixtureCsvPath = path.join(root, "validation", "results", "ipma_reference.csv");
const qplsCliPath = process.env.QUICKPLS_CLI_PATH ?? path.join(root, "target", "debug", "qpls.exe");
const disposableProjectPath = path.join(root, "validation", "results", `v247-native-methods-${Date.now()}-${process.pid}.qpls`);
const disposableProjectName = "Native Methods Acceptance";
const disposableModelName = "WPLS Structural Model";
const mediationProjectPath = path.join(root, "validation", "results", `v247-native-mediation-${Date.now()}-${process.pid}.qpls`);
const mediationProjectName = "Native Mediation Acceptance";
const mediationModelName = "Mediation Structural Model";
const moderationProjectPath = path.join(root, "validation", "results", `v247-native-moderation-${Date.now()}-${process.pid}.qpls`);
const moderationProjectName = "Native Moderation Acceptance";
const moderationModelName = "Moderation Structural Model";
const hocProjectPath = path.join(root, "validation", "results", `v247-native-hoc-${Date.now()}-${process.pid}.qpls`);
const hocProjectName = "Native Higher-Order Acceptance";
const hocModelName = "Higher-Order Structural Model";
const mgaProjectPath = path.join(root, "validation", "results", `v247-native-mga-${Date.now()}-${process.pid}.qpls`);
const mgaProjectName = "Native MGA Acceptance";
const mgaModelName = "Two-Group Structural Model";
const mgaRuntimePermutationSamples = 5_000;
const mgaMethodVersion = "pls_mga_two_group_v2";
const mgaPermutationMethodVersion = "pls_mga_permutation_v2";
const micomMethodVersion = "micom_v2";
const ccaProjectPath = path.join(root, "validation", "results", `v247-native-cca-${Date.now()}-${process.pid}.qpls`);
const ccaProjectName = "Native CCA Acceptance";
const ccaModelName = "CCA Residual Model";
const ccaMethodVersion = "cca_composite_residual_v1";
const ccaProvenanceMethodVersion = "pls_pm_v1+cca_composite_residual_v1+pls_mediation_v1+pls_assessment_v7";
const ccaNestedModelVersion = "recursive_standardized_composite_path_model_v1";
const ipmaProjectPath = path.join(root, "validation", "results", `v247-native-ipma-${Date.now()}-${process.pid}.qpls`);
const ipmaProjectName = "Native IPMA Acceptance";
const ipmaModelName = "Importance-Performance Structural Model";
const ipmaMethodVersion = "ipma_v1";
const ipmaProvenanceMethodVersion = "pls_pm_v1+ipma_v1+pls_mediation_v1+pls_assessment_v7";
const ipmaPerformanceScale = "min_max_0_100_from_standardized_scores_v1";
const ncaFixtureCsvPath = path.join(root, "validation", "results", "nca_native_reference.csv");
const ncaProjectPath = path.join(root, "validation", "results", `v247-native-nca-${Date.now()}-${process.pid}.qpls`);
const ncaProjectName = "Native NCA Acceptance";
const ncaMethodVersion = "nca_v2";
const ncaPermutationSamples = 19;
const ncaSeed = 20_260_811;
const ncaTolerance = 1e-9;
const pcaFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const pcaProjectPath = path.join(root, "validation", "results", `v247-native-pca-${Date.now()}-${process.pid}.qpls`);
const pcaProjectName = "Native PCA Acceptance";
const pcaMethodVersion = "pca_v1";
const pcaVariables = ["x", "m", "w", "y", "z"];
const pcaVarianceThreshold = 0.95;
const olsFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const olsProjectPath = path.join(root, "validation", "results", `v247-native-ols-${Date.now()}-${process.pid}.qpls`);
const olsProjectName = "Native OLS Acceptance";
const olsMethodVersion = "regression_ols_v1";
const olsOutcome = "y";
const olsPredictors = ["x", "m"];
const olsControls = ["z"];
const cbsemFixtureCsvPath = path.join(root, "validation", "results", "lavaan_latent_regression_sem.csv");
const cbsemProjectPath = path.join(root, "validation", "results", `v247-native-cbsem-${Date.now()}-${process.pid}.qpls`);
const cbsemProjectName = "Native CB-SEM Acceptance";
const cbsemModelName = "CB-SEM Structural Model";
const cbsemMethodVersion = "cbsem_ml_v1";
const cbsemFitMethodVersion = "cbsem_fit_v1";
const cbsemModificationMethodVersion = "cbsem_modification_indices_v1";
const cbsemProvenanceMethodVersion = "pls_pm_v1+cbsem_ml_v1+cbsem_fit_v1+cbsem_modification_indices_v1+pls_mediation_v1+pls_assessment_v7";
const gscaFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const gscaReferenceOutputPath = path.join(root, "validation", "results", "gsca_als_v2_quickpls.json");
const gscaProjectPath = path.join(root, "validation", "results", `v247-native-gsca-${Date.now()}-${process.pid}.qpls`);
const gscaProjectName = "Native GSCA Acceptance";
const gscaModelName = "GSCA Mixed-Block Model";
const gscaMethodVersion = "gsca_als_v2";
const gscaAlgorithmVersion = "alternating_least_squares_v1";
const predictionMethodVersion = "plspredict_indicator_v2";
const predictionRepeatedMethodVersion = "plspredict_repeated_kfold_indicator_v2";
const predictionCvpatMethodVersion = "cvpat_indicator_benchmarks_v2";
const predictionAssignment = "seeded_sha256_source_row_order_round_robin_10_v1";
const predictionFolds = 10;
const predictionRepeats = 10;
const predictionConfidenceLevel = 0.95;
const packageVersion = JSON.parse(await fs.readFile(path.join(root, "package.json"), "utf8")).version;
const requestedNativeExportPath = process.env.QUICKPLS_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedMgaNativeExportPath = process.env.QUICKPLS_MGA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedCcaNativeExportPath = process.env.QUICKPLS_CCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedIpmaNativeExportPath = process.env.QUICKPLS_IPMA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedNcaNativeExportPath = process.env.QUICKPLS_NCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPredictionNativeExportPath = process.env.QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedHocNativeExportPath = process.env.QUICKPLS_HOC_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPcaNativeExportPath = process.env.QUICKPLS_PCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedOlsNativeExportPath = process.env.QUICKPLS_OLS_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedCbsemNativeExportPath = process.env.QUICKPLS_CBSEM_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedGscaNativeExportPath = process.env.QUICKPLS_GSCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const pythonExecutable = process.env.QUICKPLS_PYTHON?.trim() || "python";

let priorEvidence = null;
if (focusedOnly) {
  try {
    priorEvidence = JSON.parse(await fs.readFile(reportPath, "utf8"));
  } catch {
    priorEvidence = null;
  }
}

const evidence = {
  passed: false,
  generatedAt: new Date().toISOString(),
  endpoint,
  runtime: "tauri-webview2-cdp",
  focusedRun: focusedOnly ? {
    scope: acceptanceScope,
    priorGeneratedAt: priorEvidence?.generatedAt ?? null,
    completedAt: null,
  } : null,
  checks: focusedOnly && priorEvidence?.checks ? { ...priorEvidence.checks } : {},
  screenshots: focusedOnly && Array.isArray(priorEvidence?.screenshots)
    ? priorEvidence.screenshots.filter((file) => acceptanceScope === "nca"
      ? !/\\(?:84|85|86|87|88|89)-tauri-native-nca-/i.test(file)
      : acceptanceScope === "prediction"
        ? !/\\9[0-7]-tauri-native-prediction-/i.test(file)
        : acceptanceScope === "hoc"
          ? !/\\10[0-6]-tauri-native-hoc-/i.test(file)
          : acceptanceScope === "pca"
            ? !/\\11[0-7]-tauri-native-pca-/i.test(file)
          : acceptanceScope === "ols"
            ? !/\\12[0-7]-tauri-native-ols-/i.test(file)
          : acceptanceScope === "cbsem"
            ? !/\\13[0-6]-tauri-native-cbsem-/i.test(file)
          : acceptanceScope === "gsca"
            ? !/\\14[0-6]-tauri-native-gsca-/i.test(file)
          : !/\\6[0-9]-tauri-native-mga-/i.test(file))
    : [],
  consoleErrors: [],
  failures: [],
};

async function writeAcceptanceEvidence() {
  const serialized = JSON.stringify(evidence, null, 2) + "\n";
  await fs.writeFile(reportPath, serialized, "utf8");
  if (scopedReportPath !== reportPath) await fs.writeFile(scopedReportPath, serialized, "utf8");
}

const expectedOptionLabels = [
  "PLS-SEM Algorithm",
  "Consistent PLS",
  "Weighted PLS",
  "GSCA",
  "CCA composite residual diagnostics",
  "Importance-Performance Map Analysis",
  "CB-SEM / CFA",
  "PLS-SEM Bootstrapping",
  "Structural Path Randomization",
  "MICOM and Two-Group Permutation MGA",
  "PLSpredict / CVPAT",
  "Necessary Condition Analysis",
  "Principal Component Analysis",
  "Ordinary Least Squares Regression",
];

await fs.mkdir(screenshotDir, { recursive: true });

async function validateRequestedNativeExportPath(value, environmentVariable = "QUICKPLS_NATIVE_EXPORT_PATH") {
  if (!path.isAbsolute(value)) {
    throw new Error(`${environmentVariable} must be an absolute .xlsx path under validation/results.`);
  }
  const targetPath = path.resolve(value);
  if (path.extname(targetPath).toLocaleLowerCase() !== ".xlsx") {
    throw new Error(`${environmentVariable} must use the .xlsx extension.`);
  }
  const realResultsDir = await fs.realpath(validationResultsDir);
  const realTargetParent = await fs.realpath(path.dirname(targetPath));
  const relativeParent = path.relative(realResultsDir, realTargetParent);
  if (relativeParent === ".." || relativeParent.startsWith(`..${path.sep}`) || path.isAbsolute(relativeParent)) {
    throw new Error(`${environmentVariable} must resolve inside validation/results.`);
  }
  try {
    await fs.access(targetPath);
    throw new Error(`${environmentVariable} must not already exist: ${targetPath}`);
  } catch (error) {
    if (error instanceof Error && error.message.startsWith(environmentVariable)) throw error;
    if (error?.code !== "ENOENT") throw error;
  }
  return targetPath;
}

function startWindowsNativeSaveExportHelper({
  targetPath,
  windowTitle,
  expectedSheets = [],
  expectedSharedStrings = [],
}) {
  const helperArguments = [
    windowsNativeSaveHelperPath,
    "--target",
    targetPath,
    "--results-root",
    validationResultsDir,
    "--window-title",
    windowTitle,
    "--timeout-seconds",
    "45",
  ];
  for (const expectedSheet of expectedSheets) {
    helperArguments.push("--expected-sheet", expectedSheet);
  }
  for (const expectedSharedString of expectedSharedStrings) {
    helperArguments.push("--expected-shared-string", expectedSharedString);
  }
  const child = spawn(pythonExecutable, helperArguments, {
    cwd: root,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });

  const events = [];
  const protocolErrors = [];
  let stdoutBuffer = "";
  let stderr = "";
  let readyResolved = false;
  let completedResolved = false;
  let exited = false;
  let finalEvent = null;
  let resolveReady;
  let resolveCompleted;
  const ready = new Promise((resolve) => { resolveReady = resolve; });
  const completed = new Promise((resolve) => { resolveCompleted = resolve; });

  const failureEvent = (message, context = {}) => ({
    event: "complete",
    passed: false,
    phase: "helper_transport",
    error: { type: "HelperTransportError", message },
    ...context,
  });
  const settleReady = (value) => {
    if (readyResolved) return;
    readyResolved = true;
    resolveReady(value);
  };
  const settleCompleted = (value) => {
    if (completedResolved) return;
    completedResolved = true;
    resolveCompleted(value);
  };
  const consumeLine = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      events.push(event);
      if (event?.event === "ready") settleReady(event);
      if (event?.event === "complete") {
        finalEvent = event;
        if (!event.passed) settleReady(event);
      }
    } catch (error) {
      protocolErrors.push({ line, error: error instanceof Error ? error.message : String(error) });
    }
  };

  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdoutBuffer += chunk;
    const lines = stdoutBuffer.split(/\r?\n/);
    stdoutBuffer = lines.pop() ?? "";
    lines.forEach(consumeLine);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });

  child.on("error", (error) => {
    const failure = failureEvent(error.message, { code: error.code ?? null });
    settleReady(failure);
    settleCompleted({ ...failure, transport: { stderr, events, protocolErrors } });
  });
  child.on("close", (code, signal) => {
    exited = true;
    if (stdoutBuffer.trim()) consumeLine(stdoutBuffer);
    let result = finalEvent ?? failureEvent("The native Save helper exited without a completion event.");
    if (result.passed && (code !== 0 || protocolErrors.length > 0)) {
      result = failureEvent("The native Save helper reported success with an invalid process or JSON-lines outcome.", {
        reportedCompletion: result,
      });
    }
    const withTransport = { ...result, transport: { exitCode: code, signal, stderr: stderr.trim(), events, protocolErrors } };
    settleReady(result.passed ? failureEvent("The native Save helper exited before its readiness event.") : result);
    settleCompleted(withTransport);
  });

  const timeout = setTimeout(() => {
    if (exited) return;
    child.kill();
    const failure = failureEvent("The native Save helper exceeded its 70 second transport timeout.");
    settleReady(failure);
    settleCompleted({ ...failure, transport: { stderr: stderr.trim(), events, protocolErrors } });
  }, 70_000);
  completed.finally(() => clearTimeout(timeout));

  return {
    ready,
    completed,
    stop: () => { if (!exited) child.kill(); },
  };
}

async function provisionDisposableProject({ sourceCsv, projectPath, projectName }) {
  try {
    await fs.access(sourceCsv);
  } catch {
    throw new Error(`The tracked native acceptance fixture is missing: ${sourceCsv}`);
  }
  try {
    await fs.access(qplsCliPath);
  } catch {
    throw new Error(`QuickPLS CLI is required to provision the disposable acceptance project. Build it first with cargo build -p qpls-cli, or set QUICKPLS_CLI_PATH. Expected: ${qplsCliPath}`);
  }
  const { stdout, stderr } = await execFileAsync(qplsCliPath, [
    "import",
    sourceCsv,
    projectPath,
    "--name",
    projectName,
  ], { cwd: root, windowsHide: true, maxBuffer: 1024 * 1024 });
  return {
    sourceCsv,
    project: projectPath,
    projectName,
    cli: qplsCliPath,
    stdout: stdout.trim(),
    stderr: stderr.trim(),
  };
}

async function provisionMgaReferenceFixture(filePath) {
  const columns = ["group", "x1", "x2", "z1", "z2", "y1", "y2"];
  const rows = [];
  const format = (value) => value.toFixed(8);
  for (let index = 0; index < 180; index += 1) {
    const observation = index + 1;
    const group = index < 90 ? "A" : "B";
    const x = Math.sin(observation * 0.37) + 0.35 * Math.cos(observation * 0.11);
    const z = Math.cos(observation * 0.29) + 0.25 * Math.sin(observation * 0.17);
    const disturbance = 0.18 * Math.sin(observation * 1.37) + 0.08 * Math.cos(observation * 0.73);
    const y = group === "A"
      ? 0.78 * x + 0.18 * z + disturbance
      : 0.24 * x + 0.72 * z + disturbance;
    rows.push([
      group,
      format(x + 0.10 * Math.sin(observation * 0.83)),
      format(0.91 * x + 0.13 * Math.cos(observation * 0.61)),
      format(z + 0.11 * Math.cos(observation * 0.79)),
      format(0.89 * z + 0.12 * Math.sin(observation * 0.67)),
      format(y + 0.09 * Math.cos(observation * 0.97)),
      format(0.93 * y + 0.10 * Math.sin(observation * 1.09)),
    ].join(","));
  }
  await fs.writeFile(filePath, `${columns.join(",")}\n${rows.join("\n")}\n`, "utf8");
  return {
    path: filePath,
    columns,
    rows: rows.length,
    groups: { A: 90, B: 90 },
    deterministic: true,
  };
}

async function provisionNcaReferenceFixture(filePath) {
  const csv = "x,y\n0,1\n1,3\n2,2\n3,4\n";
  await fs.writeFile(filePath, csv, "utf8");
  return {
    path: filePath,
    columns: ["x", "y"],
    rows: 4,
    completeCases: 4,
    deterministic: true,
    expected: {
      scope: { minimum_x: 0, maximum_x: 3, minimum_y: 1, maximum_y: 4 },
      ceFdhPeers: [{ x: 0, y: 1 }, { x: 1, y: 3 }, { x: 3, y: 4 }],
      ceFdhEffectSize: 5 / 9,
      crFdh: { slope: 13 / 14, intercept: 10 / 7, effectSize: 36 / 91 },
      ceiling: "both",
      permutationSamples: ncaPermutationSamples,
      seed: ncaSeed,
    },
  };
}

try {
  if (gscaOnly) {
    evidence.checks.gscaFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: gscaFixtureCsvPath,
      projectPath: gscaProjectPath,
      projectName: gscaProjectName,
    });
  } else if (cbsemOnly) {
    evidence.checks.cbsemFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: cbsemFixtureCsvPath,
      projectPath: cbsemProjectPath,
      projectName: cbsemProjectName,
    });
  } else if (olsOnly) {
    evidence.checks.olsFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: olsFixtureCsvPath,
      projectPath: olsProjectPath,
      projectName: olsProjectName,
    });
  } else if (pcaOnly) {
    evidence.checks.pcaFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: pcaFixtureCsvPath,
      projectPath: pcaProjectPath,
      projectName: pcaProjectName,
    });
  } else if (predictionOnly) {
    evidence.checks.fixtureProvisioning = await provisionDisposableProject({
      sourceCsv: fixtureCsvPath,
      projectPath: disposableProjectPath,
      projectName: disposableProjectName,
    });
  } else if (hocOnly) {
    evidence.checks.hocFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: hocFixtureCsvPath,
      projectPath: hocProjectPath,
      projectName: hocProjectName,
    });
  } else {
    if (!mgaOnly) {
      evidence.checks.ncaReferenceFixture = await provisionNcaReferenceFixture(ncaFixtureCsvPath);
    }
    if (!ncaOnly) {
      evidence.checks.mgaReferenceFixture = await provisionMgaReferenceFixture(mgaFixtureCsvPath);
      if (!mgaOnly) {
        evidence.checks.fixtureProvisioning = await provisionDisposableProject({
          sourceCsv: fixtureCsvPath,
          projectPath: disposableProjectPath,
          projectName: disposableProjectName,
        });
        evidence.checks.mediationFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: mediationFixtureCsvPath,
          projectPath: mediationProjectPath,
          projectName: mediationProjectName,
        });
        evidence.checks.moderationFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: moderationFixtureCsvPath,
          projectPath: moderationProjectPath,
          projectName: moderationProjectName,
        });
      }
      evidence.checks.mgaFixtureProvisioning = await provisionDisposableProject({
        sourceCsv: mgaFixtureCsvPath,
        projectPath: mgaProjectPath,
        projectName: mgaProjectName,
      });
      if (!mgaOnly) {
        evidence.checks.ccaFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: ccaFixtureCsvPath,
          projectPath: ccaProjectPath,
          projectName: ccaProjectName,
        });
        evidence.checks.ipmaFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: ipmaFixtureCsvPath,
          projectPath: ipmaProjectPath,
          projectName: ipmaProjectName,
        });
      }
    }
    if (!mgaOnly) {
      evidence.checks.ncaFixtureProvisioning = await provisionDisposableProject({
        sourceCsv: ncaFixtureCsvPath,
        projectPath: ncaProjectPath,
        projectName: ncaProjectName,
      });
      evidence.checks.ncaFixtureProvisioning.initialArchive = await inspectInitialNcaArchive(ncaProjectPath);
    }
  }
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.message : String(error));
  await writeAcceptanceEvidence();
  console.error(evidence.failures[0]);
  process.exit(1);
}

const browser = await chromium.connectOverCDP(endpoint);
const context = browser.contexts()[0];
const page = context?.pages()[0];
if (!page) throw new Error("No QuickPLS WebView2 page was available at the CDP endpoint.");

let priorRecentProjectsRaw = null;
let recentProjectsSeeded = false;
let nativeViewportLabel = "current-viewport";

page.on("pageerror", (error) => evidence.consoleErrors.push({ type: "pageerror", message: error.message }));
page.on("console", (message) => {
  if (message.type() === "error") evidence.consoleErrors.push({ type: "console", message: message.text() });
});

async function capture(name) {
  const file = path.join(screenshotDir, name);
  await page.screenshot({ path: file, animations: "disabled" });
  evidence.screenshots.push(file);
  return file;
}

async function openMenuItem(menu, item) {
  await page.getByRole("menuitem", { name: menu, exact: true }).click();
  await page.getByRole("menuitem", { name: item, exact: true }).click();
}

async function waitForSurface(surface, timeout = 15_000) {
  await page.locator(`.nd-app[data-surface="${surface}"]`).waitFor({ state: "visible", timeout });
}

async function captureActiveCalculation(dialog, name, methodLabel) {
  const progress = dialog.locator(".nd-run-progress");
  await progress.waitFor({ state: "visible", timeout: 5_000 });
  const state = await progress.evaluate((element) => ({
    status: [...element.classList].find((className) => ["queued", "validating", "running", "cancelling"].includes(className)) ?? null,
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
  }));
  if (!state.status) throw new Error(`${methodLabel} did not expose a genuine active calculation state.`);
  await capture(name);
  return state;
}

async function openResultTable(title) {
  const escapedTitle = title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const item = page.locator('.nd-result-tree [role="treeitem"]').filter({ hasText: new RegExp(`^${escapedTitle}$`) });
  await item.waitFor({ state: "visible", timeout: 15_000 });
  await item.click();
  await page.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  return page.locator(".nd-result-table tbody tr").count();
}

async function openCalculationFromToolbar() {
  const command = page.locator(".nd-commandbar button").filter({ hasText: /^Calculate/i });
  if (await command.count() !== 1) throw new Error("The active workspace did not expose exactly one generic Calculate command.");
  await command.click();
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("listbox", { name: "Available calculation methods", exact: true })
    .getByRole("option").first().waitFor({ state: "visible", timeout: 10_000 });
  return dialog;
}

async function openAnalysisFromDataToolbar() {
  const command = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
  const calculateCommands = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Calculate(?:\u2026|\.\.\.)?$/i });
  if (await command.count() !== 1 || await calculateCommands.count() !== 0) {
    throw new Error("The data-only workspace did not expose exactly one shared Analyze command without a duplicate Calculate command.");
  }
  await command.click();
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("listbox", { name: "Available calculation methods", exact: true })
    .getByRole("option").first().waitFor({ state: "visible", timeout: 10_000 });
  return dialog;
}

async function seedRecentProject(project) {
  if (!recentProjectsSeeded) {
    priorRecentProjectsRaw = await page.evaluate((key) => window.localStorage.getItem(key), recentProjectsKey);
  }
  await page.evaluate(({ key, project }) => {
    let existing = [];
    try {
      const parsed = JSON.parse(window.localStorage.getItem(key) ?? "[]");
      if (Array.isArray(parsed)) existing = parsed;
    } catch {
      existing = [];
    }
    const normalized = project.path.toLocaleLowerCase();
    window.localStorage.setItem(key, JSON.stringify([
      project,
      ...existing.filter((entry) => typeof entry?.path === "string" && entry.path.toLocaleLowerCase() !== normalized),
    ].slice(0, 8)));
  }, { key: recentProjectsKey, project });
  recentProjectsSeeded = true;
}

async function seedDisposableRecentProject() {
  await seedRecentProject({
    name: disposableProjectName,
    path: disposableProjectPath,
    openedAt: "2026-08-10T00:00:00.000Z",
  });
}

async function reloadToLauncher() {
  await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  await waitForSurface("launcher");
}

async function openRecentProject(projectName) {
  const row = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: projectName });
  await row.waitFor({ state: "visible", timeout: 10_000 });
  if (await row.count() !== 1) throw new Error(`${projectName} was not exposed as exactly one visible Recent Projects row.`);
  await row.click();
  await page.locator(".nd-window-project").filter({ hasText: projectName }).waitFor({ state: "visible", timeout: 15_000 });
}

async function openDisposableRecentProject() {
  await openRecentProject(disposableProjectName);
}

function exactVisibleText(value) {
  const escaped = value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^\\s*${escaped}\\s*$`);
}

function compactVisibleText(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function workspaceTreeItem(kind, label) {
  const item = page.locator(`.nd-project-treeitem[data-kind="${kind}"]`);
  return label ? item.filter({ hasText: exactVisibleText(label) }) : item;
}

async function openWorkspaceExplorer(projectName = disposableProjectName) {
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await page.locator(".nd-workspace-explorer").waitFor({ state: "visible", timeout: 15_000 });
  return page.getByRole("tree", { name: `${projectName} project contents`, exact: true });
}

async function submitNamedExplorerDialog(title, value, actionLabel) {
  const dialog = page.getByRole("dialog", { name: title, exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByLabel("Name", { exact: true }).fill(value);
  await dialog.getByRole("button", { name: actionLabel, exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 15_000 });
}

async function confirmExplorerRemoval(title, actionLabel) {
  const dialog = page.getByRole("dialog", { name: title, exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("button", { name: actionLabel, exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 15_000 });
}

async function createInitialEditableModel(projectName, modelName) {
  const tree = await openWorkspaceExplorer(projectName);
  await tree.waitFor({ state: "visible", timeout: 15_000 });
  const importedEditableModels = await workspaceTreeItem("model").count();
  if (importedEditableModels !== 0) {
    throw new Error(`${projectName} unexpectedly exposed ${importedEditableModels} editable models before native authoring.`);
  }
  await workspaceTreeItem("models", "Models").click();
  const newModelCommand = page.locator(".nd-explorer-detail-actions").getByRole("button", { name: "New Model", exact: true });
  if (!await newModelCommand.isEnabled()) {
    throw new Error(`${projectName} did not expose an enabled New Model action in Project Explorer.`);
  }
  await newModelCommand.click();
  await submitNamedExplorerDialog("New Model", modelName, "Create");
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const created = {
    projectName,
    importedEditableModels,
    name: (await page.locator(".nd-document-tab span").textContent())?.trim() ?? "",
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (created.name !== modelName || created.constructs !== 0 || created.structuralPaths !== 0) {
    throw new Error(`Project Explorer did not create the expected empty ${modelName} model: ${JSON.stringify(created)}`);
  }
  return created;
}

async function clickIndicator(name) {
  const indicator = page.locator(".nd-variable-item").filter({ hasText: new RegExp(`^${name}$`) });
  await indicator.waitFor({ state: "visible", timeout: 10_000 });
  if (await indicator.count() !== 1) throw new Error(`Expected exactly one visible ${name} indicator in the Model navigator.`);
  await indicator.click();
}

async function clearModelSelection() {
  const pane = page.locator(".react-flow__pane");
  await pane.waitFor({ state: "visible", timeout: 10_000 });
  const box = await pane.boundingBox();
  if (!box) throw new Error("The model canvas pane did not expose screen bounds.");
  await pane.click({ position: { x: Math.max(8, box.width - 24), y: 24 } });
  await page.locator(".react-flow__node-latent.selected").waitFor({ state: "hidden", timeout: 5_000 }).catch(() => undefined);
}

function structuralPaths() {
  return page.locator('.react-flow__edge[data-id]:not([data-id^="measurement::"])');
}

async function buildTwoConstructModel() {
  await clickIndicator("x1");
  await page.locator(".react-flow__node-latent").nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("x2");
  await clearModelSelection();
  await clickIndicator("y1");
  await page.locator(".react-flow__node-latent").nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("y2");

  const nodes = page.locator(".react-flow__node-latent");
  if (await nodes.count() !== 2) throw new Error("Visible indicator actions did not create exactly two constructs.");
  const pathCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ });
  if (await pathCommand.count() !== 1) throw new Error("The Model toolbar did not expose exactly one Path command.");
  await pathCommand.click();
  await nodes.nth(0).dispatchEvent("click");
  await nodes.nth(1).dispatchEvent("click");
  await structuralPaths().first().waitFor({ state: "attached", timeout: 10_000 });
  if (await structuralPaths().count() !== 1) throw new Error("The visible Path workflow did not create exactly one structural path.");
}

async function buildTwoConstructGscaModel() {
  const nodes = page.locator(".react-flow__node-latent");
  await clickIndicator("g1");
  await nodes.nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("g2");
  await clickIndicator("g3");
  await renameSelectedConstruct("G formative component", "G");
  const formative = page.locator('aside[aria-label="Model properties"]').getByLabel("Formative", { exact: true });
  await formative.waitFor({ state: "visible", timeout: 5_000 });
  await formative.check();
  await clearModelSelection();

  await clickIndicator("h1");
  await nodes.nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("h2");
  await renameSelectedConstruct("H reflective component", "H");
  const reflective = page.locator('aside[aria-label="Model properties"]').getByLabel("Reflective", { exact: true });
  await reflective.waitFor({ state: "visible", timeout: 5_000 });
  await reflective.check();
  await clearModelSelection();

  if (await nodes.count() !== 2) throw new Error("Visible indicator actions did not create the two GSCA components.");
  await createStructuralPath(nodes, 0, 1, 1);
}

async function createStructuralPath(nodes, sourceIndex, targetIndex, expectedPathCount) {
  const pathCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ });
  if (await pathCommand.count() !== 1) throw new Error("The Model toolbar did not expose exactly one Path command.");
  await pathCommand.click();
  await nodes.nth(sourceIndex).dispatchEvent("click");
  await nodes.nth(targetIndex).dispatchEvent("click");
  await structuralPaths().nth(expectedPathCount - 1).waitFor({ state: "attached", timeout: 10_000 });
  if (await structuralPaths().count() !== expectedPathCount) {
    throw new Error(`The visible Path workflow did not create exactly ${expectedPathCount} structural path(s).`);
  }
}

async function buildThreeConstructMediationModel() {
  await clickIndicator("x1");
  await page.locator(".react-flow__node-latent").nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("x2");
  await clearModelSelection();

  await clickIndicator("m1");
  await page.locator(".react-flow__node-latent").nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("m2");
  await clearModelSelection();

  await clickIndicator("y1");
  await page.locator(".react-flow__node-latent").nth(2).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("y2");

  const nodes = page.locator(".react-flow__node-latent");
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly three mediation constructs.");
  await createStructuralPath(nodes, 0, 1, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildThreeConstructMgaModel() {
  const definitions = [
    { indicators: ["x1", "x2"], name: "X" },
    { indicators: ["z1", "z2"], name: "Z" },
    { indicators: ["y1", "y2"], name: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    await clickIndicator(definition.indicators[1]);
    await renameSelectedConstruct(definition.name, definition.name);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, Z, and Y MGA constructs.");
  await createStructuralPath(nodes, 0, 2, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildThreeConstructCcaModel() {
  const definitions = [
    { indicators: ["x1", "x2"], name: "X" },
    { indicators: ["z1", "z2"], name: "Z" },
    { indicators: ["y1", "y2"], name: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    await clickIndicator(definition.indicators[1]);
    await renameSelectedConstruct(definition.name, definition.name);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, Z, and Y CCA constructs.");
  await createStructuralPath(nodes, 0, 1, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildThreeConstructCbsemModel() {
  const definitions = [
    { indicators: ["x1", "x2", "x3"], name: "X" },
    { indicators: ["m1", "m2", "m3"], name: "M" },
    { indicators: ["y1", "y2", "y3"], name: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    for (const indicator of definition.indicators.slice(1)) await clickIndicator(indicator);
    await renameSelectedConstruct(definition.name, definition.name);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, M, and Y CB-SEM factors.");
  await createStructuralPath(nodes, 0, 1, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildSixConstructIpmaModelWithDisconnectedBranch() {
  const definitions = [
    { indicator: "x1", name: "X" },
    { indicator: "z1", name: "Z" },
    { indicator: "m1", name: "M" },
    { indicator: "y1", name: "Y" },
    { indicator: "u1", name: "U" },
    { indicator: "v1", name: "V" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  const constructIds = {};
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicator);
    const node = nodes.nth(index);
    await node.waitFor({ state: "visible", timeout: 10_000 });
    await renameSelectedConstruct(definition.name, definition.name);
    await node.filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    const id = await node.getAttribute("data-id");
    if (!id) throw new Error(`The visible ${definition.name} IPMA construct did not expose an immutable identifier.`);
    constructIds[definition.name.toLocaleLowerCase()] = id;
    await clearModelSelection();
  }
  if (await nodes.count() !== 6) throw new Error("Visible indicator actions did not create exactly the X, Z, M, Y, U, and V IPMA constructs.");
  await createStructuralPath(nodes, 0, 2, 1);
  await createStructuralPath(nodes, 1, 2, 2);
  await createStructuralPath(nodes, 0, 3, 3);
  await createStructuralPath(nodes, 1, 3, 4);
  await createStructuralPath(nodes, 2, 3, 5);
  await createStructuralPath(nodes, 4, 5, 6);
  return { constructIds };
}

async function renameSelectedConstruct(name, shortName) {
  const properties = page.locator('aside[aria-label="Model properties"]');
  const nameInput = properties.getByLabel("Name", { exact: true });
  const shortNameInput = properties.getByLabel("Short name", { exact: true });
  await nameInput.waitFor({ state: "visible", timeout: 5_000 });
  await nameInput.fill(name);
  await nameInput.press("Enter");
  await shortNameInput.fill(shortName);
  await shortNameInput.press("Enter");
  await page.waitForFunction(({ name, shortName }) => {
    const propertiesPane = document.querySelector('aside[aria-label="Model properties"]');
    const inputs = propertiesPane?.querySelectorAll('input[type="text"]') ?? [];
    return inputs[0]?.value === name && inputs[1]?.value === shortName;
  }, { name, shortName }, { timeout: 5_000 });
}

async function buildThreeConstructModerationModel() {
  const definitions = [
    { indicator: "x", name: "X", shortName: "X" },
    { indicator: "m", name: "M", shortName: "M" },
    { indicator: "y", name: "Y", shortName: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicator);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    await renameSelectedConstruct(definition.name, definition.shortName);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, M, and Y constructs.");
  const nodeIds = await nodes.evaluateAll((elements) => elements.map((element) => element.getAttribute("data-id")));
  if (nodeIds.some((id) => !id)) throw new Error(`The X, M, and Y model nodes did not expose stable React Flow identifiers: ${JSON.stringify(nodeIds)}`);
  await createStructuralPath(nodes, 0, 2, 1);
  const basePathId = await structuralPaths().first().getAttribute("data-id");
  if (!basePathId) throw new Error("The visible X-to-Y structural relationship had no React Flow identifier.");
  return { nodes, xId: nodeIds[0], mId: nodeIds[1], yId: nodeIds[2], basePathId };
}

async function buildThreeConstructHigherOrderModel() {
  const definitions = [
    { indicator: "x1", name: "Capability", shortName: "CAP" },
    { indicator: "z1", name: "Resources", shortName: "RES" },
    { indicator: "y1", name: "Performance", shortName: "PERF" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  const ids = {};
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicator);
    const node = nodes.nth(index);
    await node.waitFor({ state: "visible", timeout: 10_000 });
    await renameSelectedConstruct(definition.name, definition.shortName);
    await node.filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    const id = await node.getAttribute("data-id");
    if (!id) throw new Error(`The visible ${definition.name} construct did not expose an immutable identifier.`);
    ids[definition.name.toLocaleLowerCase()] = id;
    await clearModelSelection();
  }
  if (await nodes.count() !== 3 || await structuralPaths().count() !== 0) {
    throw new Error("Visible indicator actions did not create exactly three measurement-only HOC input constructs.");
  }
  return { nodes, ids };
}

async function selectVisibleStructuralPath(edge) {
  await edge.waitFor({ state: "attached", timeout: 10_000 });
  if (await edge.count() !== 1) throw new Error("Expected exactly one visible structural relationship for path selection.");
  const hitTarget = edge.locator(".react-flow__edge-interaction");
  if (await hitTarget.count() !== 1) throw new Error("The structural relationship did not expose one React Flow interaction target.");
  const edgeId = await edge.getAttribute("data-id");
  if (!edgeId) throw new Error("The structural relationship did not expose a React Flow identifier for selection.");
  await hitTarget.dispatchEvent("click");
  await page.waitForFunction((selectedEdgeId) => document.querySelector(`.react-flow__edge[data-id="${CSS.escape(selectedEdgeId)}"]`)?.classList.contains("selected"), edgeId, { timeout: 5_000 });
}

async function inspectVisibleStructuralPath(edge) {
  await selectVisibleStructuralPath(edge);
  const properties = page.locator('aside[aria-label="Model properties"] .nd-property-list').first();
  await properties.waitFor({ state: "visible", timeout: 5_000 });
  return properties.evaluate((element) => Object.fromEntries(Array.from(element.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
}

const mediationBaseTableTitles = [
  "Direct effects",
  "Specific indirect effects",
  "Total indirect effects",
  "Total effects",
];
const mediationBootstrapTableTitle = "Aggregate mediation effects bootstrap inference";

async function inspectMediationResultTree({ withBootstrap }) {
  const group = page.getByRole("treeitem", { name: "Mediation", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const requiredTitles = withBootstrap
    ? [...mediationBaseTableTitles, mediationBootstrapTableTitle]
    : mediationBaseTableTitles;
  const rowCounts = {};
  const tableText = {};
  for (const title of requiredTitles) {
    rowCounts[title] = await openResultTable(title);
    tableText[title] = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
    if (!rowCounts[title] || !tableText[title]) throw new Error(`The native ${title} mediation table was empty.`);
  }

  const bootstrapTreeItems = await page.getByRole("treeitem", { name: mediationBootstrapTableTitle, exact: true }).count();
  if (withBootstrap && bootstrapTreeItems !== 1) {
    throw new Error(`The completed Bootstrap run exposed ${bootstrapTreeItems} Aggregate mediation effects bootstrap inference tree items instead of one.`);
  }
  if (!withBootstrap && bootstrapTreeItems !== 0) {
    throw new Error("The non-resampled PLS run exposed fabricated mediation bootstrap inference.");
  }

  await openResultTable("Specific indirect effects");
  return {
    groupTitle: (await group.textContent())?.trim() ?? "",
    requiredTitles,
    treeItems: (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map((label) => label.trim()),
    rowCounts,
    tableText,
    bootstrapTreeItems,
    selectedTable: (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.trim() ?? "",
  };
}

const moderationBaseTableTitles = ["Moderation effects", "Simple slope analysis"];
const moderationBootstrapTableTitle = "Interaction effect bootstrap inference";

async function inspectModerationResultTree({ withBootstrap }) {
  const group = page.getByRole("treeitem", { name: "Moderation", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const requiredTitles = withBootstrap
    ? [...moderationBaseTableTitles, moderationBootstrapTableTitle]
    : moderationBaseTableTitles;
  const rowCounts = {};
  const tableText = {};
  for (const title of requiredTitles) {
    rowCounts[title] = await openResultTable(title);
    tableText[title] = (await page.locator(".nd-result-table tbody").textContent())?.replace(/\s+/g, " ").trim() ?? "";
    if (!rowCounts[title] || !tableText[title] || /\bN\/A\b/i.test(tableText[title])) {
      throw new Error(`The native ${title} moderation table was empty or contained placeholder N/A output: ${tableText[title]}`);
    }
  }

  const bootstrapTreeItems = await page.getByRole("treeitem", { name: moderationBootstrapTableTitle, exact: true }).count();
  if (withBootstrap && bootstrapTreeItems !== 1) {
    throw new Error(`The completed moderation Bootstrap run exposed ${bootstrapTreeItems} interaction-effect inference tree items instead of one.`);
  }
  if (!withBootstrap && bootstrapTreeItems !== 0) {
    throw new Error("The non-resampled moderation PLS run exposed fabricated interaction-effect bootstrap inference.");
  }

  await openResultTable("Simple slope analysis");
  const plot = page.locator(".nd-moderation-plot");
  const plotContract = {
    figures: await plot.count(),
    accessibleSvgs: await plot.locator('svg[role="img"][aria-labelledby]').count(),
    lines: await plot.locator("polyline.slope").count(),
    points: await plot.locator("circle").count(),
    caption: (await plot.locator("figcaption").textContent())?.replace(/\s+/g, " ").trim() ?? "",
  };
  if (plotContract.figures !== 1 || plotContract.accessibleSvgs !== 1 || plotContract.lines !== 1 || plotContract.points < 3 || !/Conditional effect plot/i.test(plotContract.caption)) {
    throw new Error(`Simple slope analysis did not expose one accessible conditional-effect plot backed by the reported slope points: ${JSON.stringify(plotContract)}`);
  }

  return {
    groupTitle: (await group.textContent())?.trim() ?? "",
    requiredTitles,
    treeItems: (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map((label) => label.trim()),
    rowCounts,
    tableText,
    bootstrapTreeItems,
    plot: plotContract,
    selectedTable: (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.trim() ?? "",
  };
}

const mgaTableContracts = [
  {
    title: "Two-group sample summary",
    rows: 2,
    columns: ["Group column", "Role", "Group value", "Analyzed observations"],
  },
  {
    title: "MICOM invariance summary",
    rows: 3,
    columns: ["Construct", "Configural", "Compositional", "Partial invariance", "Equal means", "Equal variances", "Full invariance", "Confidence", "Usable permutations"],
  },
  {
    title: "MICOM Step 1 - configural invariance",
    rows: 3,
    columns: ["Construct", "Configural invariance"],
  },
  {
    title: "MICOM Step 2 - compositional invariance",
    rows: 3,
    columns: ["Construct", "Original correlation", "Lower confidence bound", "Permutation p", "Compositional invariance"],
  },
  {
    title: "MICOM Step 3 - equality of composite means",
    rows: 3,
    columns: ["Construct", "Mean A", "Mean B", "Mean difference (A - B)", "Lower confidence bound", "Upper confidence bound", "Two-tailed p", "Equal means"],
  },
  {
    title: "MICOM Step 3 - equality of composite variances",
    rows: 3,
    columns: ["Construct", "Variance A", "Variance B", "Log variance ratio (A/B)", "Lower confidence bound", "Upper confidence bound", "Two-tailed p", "Equal variances"],
  },
  {
    title: "Group path coefficients",
    rows: 4,
    columns: ["Role", "Group value", "Path", "Coefficient"],
  },
  {
    title: "Group R-square",
    rows: 2,
    columns: ["Role", "Group value", "Construct", "R²"],
  },
  {
    title: "Group outer loadings",
    rows: 12,
    columns: ["Role", "Group value", "Construct", "Indicator", "Outer loading"],
  },
  {
    title: "Group outer weights",
    rows: 12,
    columns: ["Role", "Group value", "Construct", "Indicator", "Outer weight"],
  },
  {
    title: "Group A minus Group B path differences",
    rows: 2,
    columns: ["Path", "Group A", "Coefficient A", "Group B", "Coefficient B", "A − B"],
  },
  {
    title: "Two-tailed permutation path differences",
    rows: 2,
    columns: ["Path", "A − B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
  },
  {
    title: "Group A minus Group B outer-loading differences",
    rows: 6,
    columns: ["Construct", "Indicator", "Group A", "Outer loading A", "Group B", "Outer loading B", "A - B"],
  },
  {
    title: "Group A minus Group B outer-weight differences",
    rows: 6,
    columns: ["Construct", "Indicator", "Group A", "Outer weight A", "Group B", "Outer weight B", "A - B"],
  },
  {
    title: "Two-tailed permutation outer-loading differences",
    rows: 6,
    columns: ["Construct", "Indicator", "A - B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
  },
  {
    title: "Two-tailed permutation outer-weight differences",
    rows: 6,
    columns: ["Construct", "Indicator", "A - B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
  },
];

async function inspectMgaResultTree(expectedPermutationSamples) {
  const group = page.getByRole("treeitem", { name: "Groups", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const tables = {};
  for (const contract of mgaTableContracts) {
    const rows = await openResultTable(contract.title);
    const headers = (await page.locator(".nd-result-table thead th").allTextContents()).map((value) => value.replace(/\s+/g, " ").trim());
    const rowValues = await page.locator(".nd-result-table tbody tr").evaluateAll((elements) => elements.map((row) => (
      Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    const text = (await page.locator(".nd-result-table tbody").textContent())?.replace(/\s+/g, " ").trim() ?? "";
    const viewText = (await page.locator(".nd-result-table-view").textContent())?.replace(/\s+/g, " ").trim() ?? "";
    tables[contract.title] = { rows, headers, rowValues, text, viewText };
    if (rows !== contract.rows || JSON.stringify(headers) !== JSON.stringify(contract.columns) || !text || /\bN\/A\b/i.test(text)) {
      throw new Error(`The native ${contract.title} MGA table did not match its exact non-placeholder contract: ${JSON.stringify(tables[contract.title])}`);
    }
  }

  const sample = tables["Two-group sample summary"];
  const micomSummary = tables["MICOM invariance summary"];
  const micomConfigural = tables["MICOM Step 1 - configural invariance"];
  const micomComposition = tables["MICOM Step 2 - compositional invariance"];
  const micomMeans = tables["MICOM Step 3 - equality of composite means"];
  const micomVariances = tables["MICOM Step 3 - equality of composite variances"];
  const paths = tables["Group path coefficients"];
  const loadings = tables["Group outer loadings"];
  const weights = tables["Group outer weights"];
  const rSquared = tables["Group R-square"];
  const differences = tables["Group A minus Group B path differences"];
  const loadingDifferences = tables["Group A minus Group B outer-loading differences"];
  const weightDifferences = tables["Group A minus Group B outer-weight differences"];
  const permutation = tables["Two-tailed permutation path differences"];
  const permutationLoadings = tables["Two-tailed permutation outer-loading differences"];
  const permutationWeights = tables["Two-tailed permutation outer-weight differences"];
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map((label) => label.replace(/\s+/g, " ").trim());
  const resultsWorkspaceText = (await page.locator(".nd-results-workspace").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const allGroupTableText = Object.values(tables).map((table) => table.viewText).join(" ");
  const selectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const contract = {
    groupTitle: (await group.textContent())?.replace(/\s+/g, " ").trim() ?? "",
    selectedTable,
    treeItems,
    tables,
    noPooledDiagram: !treeItems.includes("Graphical results")
      && !treeItems.includes("Model estimates")
      && await page.locator(".nd-result-diagram-view").count() === 0,
    noApproximateNormalInference: !/(?:approximate normal|normal approximation|t[- ]?statistic|standard error)/i.test(`${allGroupTableText} ${resultsWorkspaceText}`),
    allRequiredTablesVisible: mgaTableContracts.every((table) => treeItems.includes(table.title)),
    noPlaceholderNa: !/\bN\/A\b/i.test(`${allGroupTableText} ${resultsWorkspaceText}`),
  };

  if (JSON.stringify(sample.rowValues) !== JSON.stringify([
    ["group", "Group A", "A", "90"],
    ["group", "Group B", "B", "90"],
  ])) {
    throw new Error(`The two-group sample summary did not expose both 90-case A/B samples: ${sample.text}`);
  }
  if (![paths.text, differences.text, permutation.text].every((text) => /X\s*(?:→|->)\s*Y/i.test(text) && /Z\s*(?:→|->)\s*Y/i.test(text))) {
    throw new Error(`The MGA structural tables did not expose both X -> Y and Z -> Y paths: ${JSON.stringify({ paths: paths.text, differences: differences.text, permutation: permutation.text })}`);
  }
  const expectedConstructs = ["X", "Y", "Z"];
  const expectedIndicators = ["x1", "x2", "y1", "y2", "z1", "z2"];
  const tableConstructs = (table) => [...new Set(table.rowValues.map((row) => row[0]))].sort();
  const measurementIndicators = (table, index) => [...new Set(table.rowValues.map((row) => row[index]))].sort();
  const micomConstructsValid = [micomSummary, micomConfigural, micomComposition, micomMeans, micomVariances]
    .every((table) => JSON.stringify(tableConstructs(table)) === JSON.stringify(expectedConstructs));
  const groupMeasurementRowsValid = [loadings, weights].every((table) => (
    JSON.stringify(measurementIndicators(table, 3)) === JSON.stringify(expectedIndicators)
    && table.rowValues.filter((row) => row[0] === "Group A" && row[1] === "A").length === 6
    && table.rowValues.filter((row) => row[0] === "Group B" && row[1] === "B").length === 6
  ));
  const differenceMeasurementRowsValid = [loadingDifferences, weightDifferences].every((table) => (
    JSON.stringify(measurementIndicators(table, 1)) === JSON.stringify(expectedIndicators)
    && table.rowValues.every((row) => row[2] === "A" && row[4] === "B")
  ));
  const micomSummaryValid = micomSummary.rowValues.every((row) => (
    row[1] === "Confirmed"
    && ["Established", "Not established"].includes(row[2])
    && row[3] === row[2]
    && ["Equal", "Different"].includes(row[4])
    && ["Equal", "Different"].includes(row[5])
    && ["Established", "Not established"].includes(row[6])
    && /^95(?:\.0)?%$/.test(row[7])
    && row[8] === String(expectedPermutationSamples)
  ));
  const micomConfiguralValid = micomConfigural.rowValues.every((row) => row[1] === "Confirmed");
  const visibleProbability = (value) => /^(?:<0\.0001|0(?:\.\d+)?|1(?:\.0+)?)$/.test(value);
  const micomCompositionValid = micomComposition.rowValues.every((row) => (
    parseVisibleNumber(row[1]) !== null
    && parseVisibleNumber(row[2]) !== null
    && visibleProbability(row[3])
    && ["Established", "Not established"].includes(row[4])
  ));
  const micomStep3Valid = [micomMeans, micomVariances].every((table) => table.rowValues.every((row) => (
    row.slice(1, 6).every((value) => parseVisibleNumber(value) !== null)
    && visibleProbability(row[6])
    && ["Equal", "Different"].includes(row[7])
  )));
  const rSquaredConstructIndex = rSquared.headers.indexOf("Construct");
  const rSquaredOutcomeValid = rSquaredConstructIndex >= 0
    && rSquared.rowValues.length === 2
    && rSquared.rowValues.every((row) => row[rSquaredConstructIndex] === "Y");
  const permutationCountsValid = [permutation, permutationLoadings, permutationWeights].every((table) => {
    const requested = table.headers.indexOf("Requested permutations");
    const usable = table.headers.indexOf("Usable permutations");
    return requested >= 0 && usable >= 0 && table.rowValues.every((row) => (
      row[requested] === String(expectedPermutationSamples)
      && row[usable] === String(expectedPermutationSamples)
    ));
  });
  const permutationMeasurementRowsValid = [permutationLoadings, permutationWeights].every((table) => (
    JSON.stringify(measurementIndicators(table, 1)) === JSON.stringify(expectedIndicators)
    && table.rowValues.every((row) => visibleProbability(row[3]))
  ));
  if (!rSquaredOutcomeValid || !permutationCountsValid || !micomConstructsValid
    || !groupMeasurementRowsValid || !differenceMeasurementRowsValid || !permutationMeasurementRowsValid
    || !micomSummaryValid || !micomConfiguralValid || !micomCompositionValid || !micomStep3Valid) {
    throw new Error(`The MICOM v2 and permutation MGA v2 tables did not expose complete, finite, exact-sample group inference: ${JSON.stringify({ rSquared: rSquared.text, micomSummary: micomSummary.rowValues, permutationRows: permutation.rowValues })}`);
  }
  if (!contract.noPooledDiagram || !contract.noApproximateNormalInference || !contract.allRequiredTablesVisible || !contract.noPlaceholderNa) {
    throw new Error(`The completed MICOM/MGA Results workspace omitted a required v2 table or exposed a pooled, approximate-normal, or placeholder surface: ${JSON.stringify(contract)}`);
  }
  await openResultTable("MICOM invariance summary");
  return contract;
}

function parseVisibleNumber(value) {
  const normalized = value.replace(/\u2212/g, "-").replace(/,/g, "").trim();
  const number = Number(normalized);
  return Number.isFinite(number) ? number : null;
}

async function inspectCcaResultTree() {
  const assessment = page.getByRole("treeitem", { name: "Assessment", exact: true });
  await assessment.waitFor({ state: "visible", timeout: 15_000 });
  if (await assessment.getAttribute("aria-expanded") === "false") await assessment.click();

  const initialSelectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const summaryRowCount = await openResultTable("Residual summary");
  const summaryHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map((value) => value.replace(/\s+/g, " ").trim());
  const summaryRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const summary = Object.fromEntries(summaryRows.map((row) => [row[0], row[1]]));

  const residualRowCount = await openResultTable("Composite residuals");
  const residualHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map((value) => value.replace(/\s+/g, " ").trim());
  const residualRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const pairLabels = residualRows.map((row) => row[0]).sort();
  const expectedPairLabels = ["X ↔ Y", "X ↔ Z", "Z ↔ Y"].sort();
  const finiteAndConsistent = residualRows.every((row) => {
    const [observed, reproduced, residual, absoluteResidual] = row.slice(1).map(parseVisibleNumber);
    return [observed, reproduced, residual, absoluteResidual].every((value) => value !== null)
      && Math.abs(observed - reproduced - residual) <= 0.000002
      && Math.abs(Math.abs(residual) - absoluteResidual) <= 0.000002;
  });
  const maximumAbsoluteResidual = parseVisibleNumber(summary["Maximum absolute residual"] ?? "");
  const detailMaximum = Math.max(...residualRows.map((row) => parseVisibleNumber(row[4]) ?? Number.NEGATIVE_INFINITY));
  const ctaOrInferenceTreeItems = await page.locator('.nd-result-tree [role="treeitem"]')
    .filter({ hasText: /threshold|classification|p-value|confidence interval|bootstrap/i }).count();
  const renderedCcaText = `${summaryRows.flat().join(" ")} ${residualRows.flat().join(" ")}`;
  const contract = {
    groupTitle: (await assessment.textContent())?.replace(/\s+/g, " ").trim() ?? "",
    initialSelectedTable,
    summary: { headers: summaryHeaders, rows: summaryRows, rowCount: summaryRowCount },
    residuals: { headers: residualHeaders, rows: residualRows, rowCount: residualRowCount },
    nestedModelLabel: summary.Model ?? "",
    correlationPairs: Number(summary["Correlation pairs"] ?? Number.NaN),
    maximumAbsoluteResidual,
    pairLabels,
    finiteAndConsistent,
    maximumMatchesRows: maximumAbsoluteResidual !== null && Number.isFinite(detailMaximum)
      && Math.abs(maximumAbsoluteResidual - detailMaximum) <= 0.000001,
    noInventedInferenceOrClassification: ctaOrInferenceTreeItems === 0
      && !/threshold|pass\/fail|fit classification|p[- ]?value|confidence interval|bootstrap/i.test(renderedCcaText),
  };

  if (initialSelectedTable !== "Residual summary"
    || summaryRowCount !== 3
    || JSON.stringify(summaryHeaders) !== JSON.stringify(["Metric", "Value"])
    || summary.Model !== "Recursive standardized composite path model"
    || summary["Correlation pairs"] !== "3"
    || maximumAbsoluteResidual === null
    || maximumAbsoluteResidual <= 0
    || residualRowCount !== 3
    || JSON.stringify(residualHeaders) !== JSON.stringify(["Composite pair", "Observed correlation", "Reproduced correlation", "Residual", "Absolute residual"])
    || JSON.stringify(pairLabels) !== JSON.stringify(expectedPairLabels)
    || !finiteAndConsistent
    || !contract.maximumMatchesRows
    || !contract.noInventedInferenceOrClassification) {
    throw new Error(`The completed CCA result did not match its exact finite descriptive residual contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectIpmaResultTree() {
  const group = page.getByRole("treeitem", { name: "Importance-performance map", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const initialSelectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]')
    .textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const readTable = async (title) => {
    const rowCount = await openResultTable(title);
    const headers = (await page.locator(".nd-result-table thead th").allTextContents())
      .map((value) => value.replace(/\s+/g, " ").trim());
    const rows = await page.locator(".nd-result-table tbody tr").evaluateAll((elements) => elements.map((row) => (
      Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    return { title, rowCount, headers, rows };
  };

  const constructs = await readTable("Construct importance and performance");
  const plot = page.locator(".nd-ipma-plot");
  const plotSvg = plot.locator('svg[role="img"][aria-labelledby]');
  const directPlotTitles = plotSvg.locator(":scope > title");
  const directPlotTitleCount = await directPlotTitles.count();
  const captionTitles = plot.locator("figcaption > strong");
  const captionTargets = plot.locator("figcaption > span");
  const captionTitleCount = await captionTitles.count();
  const captionTargetCount = await captionTargets.count();
  const plotContract = {
    figures: await plot.count(),
    accessibleSvgs: await plotSvg.count(),
    directTitleCount: directPlotTitleCount,
    title: directPlotTitleCount === 1
      ? compactVisibleText(await directPlotTitles.textContent())
      : "",
    captionTitleCount,
    captionTargetCount,
    captionTitle: captionTitleCount === 1
      ? compactVisibleText(await captionTitles.textContent())
      : "",
    captionTarget: captionTargetCount === 1
      ? compactVisibleText(await captionTargets.textContent())
      : "",
    points: await plot.locator("circle").count(),
    pointLabels: (await plot.locator(".point-label").allTextContents()).map(compactVisibleText).sort(),
    scope: compactVisibleText(await plot.locator(":scope > p").textContent().catch(() => "")),
  };
  const indicators = await readTable("Indicator performance");
  const scope = await readTable("Calculation scope");
  const scopeValues = Object.fromEntries(scope.rows.map((row) => [row[0], row[1]]));
  const constructLabels = constructs.rows.map((row) => row[1]).sort();
  const indicatorConstructLabels = indicators.rows.map((row) => row[1]).sort();
  const indicatorLabels = indicators.rows.map((row) => row[2]).sort();
  const excludedConstructLabels = ["U", "V", "Y"];
  const excludedIndicatorLabels = ["u1", "v1", "y1"];
  const constructValuesFinite = constructs.rows.every((row) => {
    const importance = parseVisibleNumber(row[2]);
    const performance = parseVisibleNumber(row[3]);
    return importance !== null && performance !== null && performance >= 0 && performance <= 100;
  });
  const indicatorValuesFinite = indicators.rows.every((row) => {
    const values = [row[3], row[4], row[5], row[6]].map(parseVisibleNumber);
    return values.every((value) => value !== null)
      && values[2] >= 0 && values[2] <= 100;
  });
  const renderedText = [constructs, indicators, scope]
    .flatMap((table) => [table.headers, ...table.rows]).flat().join(" ")
    + ` ${Object.values(plotContract).join(" ")}`;
  const inferenceTreeItems = await page.locator('.nd-result-tree [role="treeitem"]')
    .filter({ hasText: /bootstrap|permutation|confidence interval|p-value|cIPMA/i }).count();
  const contract = {
    groupTitle: compactVisibleText(await group.textContent()),
    initialSelectedTable,
    treeItems: (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText),
    constructs,
    indicators,
    scope,
    scopeValues,
    plot: plotContract,
    constructLabels,
    indicatorConstructLabels,
    indicatorLabels,
    constructValuesFinite,
    indicatorValuesFinite,
    predecessorOnly: JSON.stringify(constructLabels) === JSON.stringify(["M", "X", "Z"])
      && JSON.stringify(indicatorConstructLabels) === JSON.stringify(["M", "X", "Z"])
      && excludedConstructLabels.every((label) => !constructLabels.includes(label)
        && !indicatorConstructLabels.includes(label)),
    excludesTargetAndUnrelatedConstructRows: excludedConstructLabels.every((label) => !constructLabels.includes(label)),
    excludesTargetAndUnrelatedIndicatorRows: excludedConstructLabels.every((label) => !indicatorConstructLabels.includes(label))
      && excludedIndicatorLabels.every((label) => !indicatorLabels.includes(label)),
    noPlaceholderOrUnsupportedClaims: inferenceTreeItems === 0
      && !/\bN\/A\b|\bcIPMA\b|p[- ]?value|confidence interval|bootstrap|permutation/i.test(renderedText),
  };

  if (initialSelectedTable !== "Construct importance and performance"
    || constructs.rowCount !== 3
    || JSON.stringify(constructs.headers) !== JSON.stringify(["Target", "Predecessor construct", "Total importance", "Performance"])
    || !constructs.rows.every((row) => row[0] === "Y")
    || !contract.predecessorOnly
    || !constructValuesFinite
    || indicators.rowCount !== 3
    || JSON.stringify(indicators.headers) !== JSON.stringify(["Target", "Construct", "Indicator", "Construct importance", "Loading", "Performance", "Standardized score mean"])
    || !indicators.rows.every((row) => row[0] === "Y")
    || JSON.stringify(indicatorLabels) !== JSON.stringify(["m1", "x1", "z1"])
    || !indicatorValuesFinite
    || !contract.excludesTargetAndUnrelatedConstructRows
    || !contract.excludesTargetAndUnrelatedIndicatorRows
    || scope.rowCount !== 5
    || JSON.stringify(scope.headers) !== JSON.stringify(["Field", "Value"])
    || scopeValues.Target !== "Y"
    || scopeValues["Method version"] !== ipmaMethodVersion
    || !/observed-range|min(?:-|\s)?max/i.test(scopeValues.Performance ?? "")
    || scopeValues["Missing data"] !== "Listwise deletion"
    || scopeValues["Theoretical-range correction"] !== "Not applied"
    || plotContract.figures !== 1
    || plotContract.accessibleSvgs !== 1
    || plotContract.directTitleCount !== 1
    || plotContract.title !== "Importance-performance map for Y"
    || plotContract.captionTitleCount !== 1
    || plotContract.captionTargetCount !== 1
    || plotContract.captionTitle !== "Importance-performance map"
    || plotContract.captionTarget !== "Target: Y"
    || plotContract.points !== 3
    || JSON.stringify(plotContract.pointLabels) !== JSON.stringify(["M", "X", "Z"])
    || !plotContract.scope.includes("observed-range")
    || !plotContract.scope.includes("listwise-standardized composite scores")
    || !plotContract.scope.includes("No theoretical-range correction is applied")
    || !contract.noPlaceholderOrUnsupportedClaims) {
    throw new Error(`The completed IPMA result did not match its exact predecessor-only descriptive map contract: ${JSON.stringify(contract)}`);
  }
  await openResultTable("Construct importance and performance");
  return contract;
}

async function inspectNcaResultTree() {
  const group = page.getByRole("treeitem", { name: "Necessary conditions", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const initialSelectedTable = compactVisibleText(await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent());
  const readTable = async (title) => {
    const rowCount = await openResultTable(title);
    const headers = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
    const rows = await page.locator(".nd-result-table tbody tr").evaluateAll((elements) => elements.map((row) => (
      Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    return { title, rowCount, headers, rows };
  };

  const effects = await readTable("Ceiling effect sizes and permutation inference");
  const plot = page.locator(".nd-nca-plot");
  const plotSvg = plot.locator('svg[role="img"][aria-labelledby]');
  const directTitle = plotSvg.locator(":scope > title");
  const description = plotSvg.locator(":scope > desc");
  const plotContract = {
    figures: await plot.count(),
    accessibleSvgs: await plotSvg.count(),
    namedImages: await plot.getByRole("img", { name: /Necessary condition ceiling plot for x and y/i }).count(),
    labelledBy: await plotSvg.getAttribute("aria-labelledby"),
    directTitleCount: await directTitle.count(),
    title: compactVisibleText(await directTitle.textContent().catch(() => "")),
    descriptionCount: await description.count(),
    description: compactVisibleText(await description.textContent().catch(() => "")),
    captionTitle: compactVisibleText(await plot.locator("figcaption > strong").textContent().catch(() => "")),
    captionPair: compactVisibleText(await plot.locator("figcaption > span").textContent().catch(() => "")),
    ceFdhPaths: await plot.locator(".ceiling.ce-fdh").count(),
    crFdhLines: await plot.locator(".ceiling.cr-fdh").count(),
    ceFdhPeers: await plot.locator("circle.ce-peer").count(),
  };
  const crLine = await readTable("CR-FDH ceiling coefficients");
  const bottlenecks = await readTable("Observed-range bottlenecks");
  const scope = await readTable("Calculation scope");
  const scopeValues = Object.fromEntries(scope.rows.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const expectedTreeItems = [
    "Necessary conditions",
    "Ceiling effect sizes and permutation inference",
    "CR-FDH ceiling coefficients",
    "Observed-range bottlenecks",
    "Calculation scope",
  ];
  const effectValues = Object.fromEntries(effects.rows.map((row) => [row[0], {
    effectSize: parseVisibleNumber(row[1]),
    permutationP: parseVisibleNumber(row[2]),
  }]));
  const pValueLattice = Object.values(effectValues).every((row) => row.permutationP !== null
    && row.permutationP >= 1 / (ncaPermutationSamples + 1) - ncaTolerance
    && row.permutationP <= 1 + ncaTolerance
    && Math.abs(row.permutationP * (ncaPermutationSamples + 1) - Math.round(row.permutationP * (ncaPermutationSamples + 1))) <= ncaTolerance);
  const displayedNumberClose = (actual, expected, tolerance) => Number.isFinite(actual)
    && Math.abs(actual - expected) <= tolerance;
  const expectedCeRequirements = [
    "33.3333% of observed X range", "33.3333% of observed X range", "33.3333% of observed X range",
    "33.3333% of observed X range", "33.3333% of observed X range", "33.3333% of observed X range",
    "100.0000% of observed X range", "100.0000% of observed X range", "100.0000% of observed X range",
  ];
  const expectedCrRequirements = [
    "Not necessary", "6.1538% of observed X range", "16.9231% of observed X range",
    "27.6923% of observed X range", "38.4615% of observed X range", "49.2308% of observed X range",
    "60.0000% of observed X range", "70.7692% of observed X range", "81.5385% of observed X range",
  ];
  const ceRows = bottlenecks.rows.filter((row) => row[0] === "CE-FDH");
  const crRows = bottlenecks.rows.filter((row) => row[0] === "CR-FDH");
  const bottlenecksMatch = ceRows.length === 9 && crRows.length === 9
    && ceRows.every((row, index) => row[1] === `${(index + 1) * 10}%` && row[2] === expectedCeRequirements[index])
    && crRows.every((row, index) => row[1] === `${(index + 1) * 10}%` && row[2] === expectedCrRequirements[index]);
  const renderedText = [effects, crLine, bottlenecks, scope]
    .flatMap((table) => [table.headers, ...table.rows]).flat().join(" ")
    + ` ${Object.values(plotContract).join(" ")}`;
  const contract = {
    groupTitle: compactVisibleText(await group.textContent()),
    initialSelectedTable,
    treeItems,
    effects,
    crLine,
    bottlenecks,
    scope,
    scopeValues,
    plot: plotContract,
    effectValues,
    pValueLattice,
    bottlenecksMatch,
    noModelOrQualityTree: !treeItems.some((label) => ["Graphical results", "Model estimates", "Quality criteria"].includes(label))
      && await page.locator(".nd-result-diagram-view").count() === 0,
    noPlaceholder: !/\bN\/A\b/i.test(renderedText),
    noBroaderNcaClaim: !/multiple conditions (?:are )?(?:included|supported)|latent-score NCA (?:is )?(?:included|supported)|cIPMA (?:is )?(?:included|supported)/i.test(renderedText),
  };

  if (initialSelectedTable !== "Ceiling effect sizes and permutation inference"
    || JSON.stringify(treeItems) !== JSON.stringify(expectedTreeItems)
    || effects.rowCount !== 2
    || JSON.stringify(effects.headers) !== JSON.stringify(["Ceiling line", "Effect size", "Permutation p"])
    || JSON.stringify(effects.rows.map((row) => row[0])) !== JSON.stringify(["CE-FDH", "CR-FDH"])
    || !displayedNumberClose(effectValues["CE-FDH"]?.effectSize, 5 / 9, 0.00005)
    || !displayedNumberClose(effectValues["CR-FDH"]?.effectSize, 36 / 91, 0.00005)
    || !pValueLattice
    || crLine.rowCount !== 1
    || JSON.stringify(crLine.headers) !== JSON.stringify(["Ceiling line", "Slope", "Intercept"])
    || crLine.rows[0]?.[0] !== "CR-FDH"
    || !displayedNumberClose(parseVisibleNumber(crLine.rows[0]?.[1] ?? ""), 13 / 14, 0.00005)
    || !displayedNumberClose(parseVisibleNumber(crLine.rows[0]?.[2] ?? ""), 10 / 7, 0.00005)
    || bottlenecks.rowCount !== 18
    || JSON.stringify(bottlenecks.headers) !== JSON.stringify(["Ceiling line", "Outcome (% observed range)", "Condition requirement"])
    || !bottlenecksMatch
    || scope.rowCount !== 10 || JSON.stringify(scope.headers) !== JSON.stringify(["Field", "Value"])
    || scopeValues["Condition variable (X)"] !== "x" || scopeValues["Outcome variable (Y)"] !== "y"
    || scopeValues["Analyzed observations"] !== "4" || scopeValues["X observed range"] !== "0.000000 to 3.000000"
    || scopeValues["Y observed range"] !== "1.000000 to 4.000000" || scopeValues["Ceiling lines"] !== "CE-FDH and CR-FDH"
    || scopeValues["Requested permutations"] !== String(ncaPermutationSamples)
    || scopeValues["Usable permutations"] !== String(ncaPermutationSamples)
    || scopeValues["Missing data"] !== "Listwise deletion" || scopeValues["Method version"] !== ncaMethodVersion
    || plotContract.figures !== 1 || plotContract.accessibleSvgs !== 1 || plotContract.namedImages !== 1
    || plotContract.labelledBy !== "nd-nca-plot-title nd-nca-plot-description" || plotContract.directTitleCount !== 1
    || plotContract.title !== "Necessary condition ceiling plot for x and y" || plotContract.descriptionCount !== 1
    || plotContract.captionTitle !== "Necessary condition ceiling plot" || plotContract.captionPair !== "x \u2192 y"
    || plotContract.ceFdhPaths !== 2 || plotContract.crFdhLines !== 2 || plotContract.ceFdhPeers !== 3
    || !plotContract.description.includes("CE-FDH peer 0, 1")
    || !plotContract.description.includes("CE-FDH peer 1, 3")
    || !plotContract.description.includes("CE-FDH peer 3, 4")
    || !plotContract.description.includes("CR-FDH slope 0.9286 and intercept 1.4286")
    || !contract.noModelOrQualityTree || !contract.noPlaceholder || !contract.noBroaderNcaClaim) {
    throw new Error(`The completed NCA result did not match its exact standalone nca_v2 tables and accessible ceiling plot: ${JSON.stringify(contract)}`);
  }
  await openResultTable("Ceiling effect sizes and permutation inference");
  return contract;
}

async function inspectCurrentRunDetails() {
  const command = page.locator(".nd-commandbar button").filter({ hasText: /^Run Details and Log/ });
  if (await command.count() !== 1 || !await command.isEnabled()) {
    throw new Error("The selected completed run did not expose exactly one enabled Run Details and Log command.");
  }
  await command.click();
  const dialog = page.getByRole("dialog", { name: "Run Details", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  const properties = await dialog.locator(".nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const logEntries = await dialog.locator(".nd-run-details ol li").count();
  await dialog.getByRole("button", { name: "Close dialog", exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 10_000 });
  return { properties, logEntries };
}

async function inspectSavedPredictionArchive(projectPath, runId) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 64 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved prediction archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = project.layouts?.workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const usedObservations = estimation?.used_observations;
  const predict = estimation?.predict;
  const repeated = predict?.repeated_kfold;
  const indicatorRows = Array.isArray(repeated?.indicator_targets) ? repeated.indicator_targets : [];
  const constructRows = Array.isArray(repeated?.targets) ? repeated.targets : [];
  const cvpatRows = Array.isArray(repeated?.cvpat_benchmark_assessments) ? repeated.cvpat_benchmark_assessments : [];
  const endogenousConstructs = Array.isArray(recipe?.model?.constructs)
    ? recipe.model.constructs.filter((construct) => recipe.model.paths?.some((path) => path.target === construct.id))
    : [];
  const expectedIndicators = endogenousConstructs.flatMap((construct) => construct.indicators ?? []).sort();
  const exactVersions = result.provenance?.method_version === `pls_pm_v1+${predictionMethodVersion}+pls_mediation_v1+pls_assessment_v7`
    && estimation?.method_version === predictionMethodVersion
    && predict?.method_version === predictionMethodVersion
    && predict?.primary_analysis === predictionRepeatedMethodVersion
    && repeated?.method_version === predictionRepeatedMethodVersion
    && cvpatRows.every((row) => row.method_version === predictionCvpatMethodVersion);
  const exactRepeatedPlan = repeated?.folds === predictionFolds
    && repeated?.repeats === predictionRepeats
    && repeated?.seed === recipe?.settings?.seed
    && repeated?.assignment === predictionAssignment
    && /^sha256:[0-9a-f]{64}$/.test(repeated?.assignment_digest ?? "")
    && repeated?.total_test_observations === usedObservations * predictionRepeats;
  const exactIndicatorRows = indicatorRows.length === expectedIndicators.length
    && JSON.stringify(indicatorRows.map((row) => row.indicator).sort()) === JSON.stringify(expectedIndicators)
    && indicatorRows.every((row) => row.predictor_scope === "earliest_antecedent_indicators"
      && Number.isInteger(row.predictor_count) && row.predictor_count > 0
      && Number.isFinite(row.q_squared_predict)
      && row.pls?.observations === usedObservations * predictionRepeats
      && row.indicator_average?.observations === usedObservations * predictionRepeats
      && ["available", "unavailable"].includes(row.linear_model?.status));
  const exactCvpatRows = cvpatRows.length === 2
    && JSON.stringify(cvpatRows.map((row) => row.benchmark).sort()) === JSON.stringify(["indicator_average", "linear_model"])
    && cvpatRows.every((row) => row.comparison_kind === "benchmark_assessment"
      && row.target_scope === "all_endogenous_indicators"
      && row.loss === "mean_squared_error_across_indicators_per_observation"
      && row.alternative === "pls_loss_less_than_benchmark"
      && row.confidence_level === predictionConfidenceLevel
      && row.observations === usedObservations
      && row.indicator_count === expectedIndicators.length
      && ["available", "inferential_test_unavailable", "benchmark_unavailable"].includes(row.status));
  const contract = {
    resultStatus: result.status ?? null,
    resultMethod: result.provenance?.method ?? null,
    resultMethodVersion: result.provenance?.method_version ?? null,
    usedObservations: usedObservations ?? null,
    exactVersions,
    exactRepeatedPlan,
    exactIndicatorRows,
    exactCvpatRows,
    indicatorCount: indicatorRows.length,
    constructCount: constructRows.length,
    cvpatBenchmarks: cvpatRows.map((row) => row.benchmark),
    noLegacyRelabel: Array.isArray(repeated?.cvpat) && repeated.cvpat.length === 0
      && Array.isArray(repeated?.paired_loss_diagnostics) && repeated.paired_loss_diagnostics.length === 0,
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      method: recipe.settings?.method ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      seed: recipe.settings?.seed ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
    } : null,
    run: run ? {
      method: run.method ?? null,
      status: run.status ?? null,
      modelId: run.modelId ?? null,
      snapshotNodes: run.modelSnapshot?.nodes?.length ?? null,
      logs: run.logs?.length ?? 0,
    } : null,
  };
  if (contract.resultStatus !== "completed" || contract.resultMethod !== "predict"
    || contract.usedObservations !== 128 || !contract.exactVersions || !contract.exactRepeatedPlan
    || !contract.exactIndicatorRows || !contract.exactCvpatRows || contract.indicatorCount !== 2
    || contract.constructCount !== 1 || !contract.noLegacyRelabel
    || contract.recipe?.status !== "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope"
    || contract.recipe?.method !== "predict" || contract.recipe?.confidenceLevel !== predictionConfidenceLevel
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || !Number.isInteger(contract.recipe?.seed)
    || contract.recipe?.constructs !== 2 || contract.recipe?.paths !== 1
    || contract.run?.method !== "PLSpredict / CVPAT" || contract.run?.status !== "completed"
    || !contract.run?.modelId || contract.run?.snapshotNodes !== 2 || contract.run.logs < 1) {
    throw new Error(`The saved prediction archive did not retain the exact current indicator-level PLSpredict/CVPAT contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedMgaArchive(projectPath, runId) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 64 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved MICOM/MGA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = project.layouts?.workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const mga = estimation?.mga;
  const permutation = estimation?.mga_permutation;
  const micom = estimation?.micom;
  const groups = Array.isArray(mga?.groups) ? mga.groups : [];
  const measurementComparisons = Array.isArray(mga?.measurement_comparisons) ? mga.measurement_comparisons : [];
  const permutationMeasurementComparisons = Array.isArray(permutation?.measurement_comparisons) ? permutation.measurement_comparisons : [];
  const micomConstructs = Array.isArray(micom?.constructs) ? micom.constructs : [];
  const provenanceTokens = String(result.provenance?.method_version ?? "").split("+");
  const exactCurrentVersions = [mgaMethodVersion, mgaPermutationMethodVersion, micomMethodVersion]
    .every((version) => provenanceTokens.filter((token) => token === version).length === 1)
    && !provenanceTokens.some((token) => /(?:pls_mga_two_group|pls_mga_permutation|micom)_v1$/.test(token));
  const exactGroupPayload = groups.length === 2
    && groups[0]?.group === "A" && groups[0]?.observations === 90
    && groups[1]?.group === "B" && groups[1]?.observations === 90
    && groups.every((group) => Array.isArray(group.paths) && group.paths.length === 2
      && Array.isArray(group.outer_estimates) && group.outer_estimates.length === 6
      && Array.isArray(group.transforms) && group.transforms.length === 6)
    && Array.isArray(mga?.comparisons) && mga.comparisons.length === 2
    && measurementComparisons.filter((row) => row.parameter === "outer_loading").length === 6
    && measurementComparisons.filter((row) => row.parameter === "outer_weight").length === 6;
  const exactPermutationPayload = permutation?.method_version === mgaPermutationMethodVersion
    && permutation?.permutation_samples === mgaRuntimePermutationSamples
    && permutation?.usable_permutations === mgaRuntimePermutationSamples
    && Number.isInteger(permutation?.attempted_permutations)
    && Number.isInteger(permutation?.failed_permutations)
    && permutation.attempted_permutations - permutation.usable_permutations === permutation.failed_permutations
    && Array.isArray(permutation?.comparisons) && permutation.comparisons.length === 2
    && permutationMeasurementComparisons.filter((row) => row.parameter === "outer_loading").length === 6
    && permutationMeasurementComparisons.filter((row) => row.parameter === "outer_weight").length === 6;
  const exactMicomPayload = micom?.method_version === micomMethodVersion
    && micom?.permutation_samples === mgaRuntimePermutationSamples
    && micom?.usable_permutations === mgaRuntimePermutationSamples
    && micom?.confidence_level === 0.95
    && Number.isInteger(micom?.attempted_permutations)
    && Number.isInteger(micom?.failed_permutations)
    && micom.attempted_permutations - micom.usable_permutations === micom.failed_permutations
    && Array.isArray(micom?.groups) && micom.groups.length === 2
    && micom.groups[0]?.group === "A" && micom.groups[0]?.observations === 90
    && micom.groups[1]?.group === "B" && micom.groups[1]?.observations === 90
    && micomConstructs.length === 3
    && micomConstructs.every((row) => row.configural_invariance === true && [
      row.compositional_correlation,
      row.compositional_correlation_lower,
      row.compositional_p_value,
      row.mean_a,
      row.mean_b,
      row.mean_difference,
      row.mean_difference_lower,
      row.mean_difference_upper,
      row.mean_p_value,
      row.variance_a,
      row.variance_b,
      row.variance_difference,
      row.variance_difference_lower,
      row.variance_difference_upper,
      row.variance_p_value,
    ].every(Number.isFinite)
      && [row.partial_invariance, row.equal_means, row.equal_variances, row.full_invariance].every((value) => typeof value === "boolean"));
  const contract = {
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    exactCurrentVersions,
    estimationMethodVersion: estimation?.method_version ?? null,
    mgaMethodVersion: mga?.method_version ?? null,
    permutationMethodVersion: permutation?.method_version ?? null,
    micomMethodVersion: micom?.method_version ?? null,
    exactGroupPayload,
    exactPermutationPayload,
    exactMicomPayload,
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      groupMethods: recipe.metadata?.group_methods ?? null,
      groupPermutationSamples: recipe.metadata?.group_permutation_samples ?? null,
      configuralConfirmed: recipe.metadata?.micom_configural_confirmed ?? null,
      groupColumn: recipe.metadata?.mga_group_column ?? null,
      groupA: recipe.metadata?.mga_group_a ?? null,
      groupB: recipe.metadata?.mga_group_b ?? null,
      method: recipe.settings?.method ?? null,
      weighting: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
    } : null,
    run: run ? { method: run.method, status: run.status, logs: run.logs?.length ?? 0 } : null,
  };
  if (contract.resultStatus !== "completed" || contract.provenanceMethod !== "mga"
    || !contract.exactCurrentVersions || contract.estimationMethodVersion !== mgaMethodVersion
    || contract.mgaMethodVersion !== mgaMethodVersion || !contract.exactGroupPayload
    || !contract.exactPermutationPayload || !contract.exactMicomPayload
    || contract.recipe?.status !== "validated_micom_v2_and_permutation_mga_v2_bounded_scope"
    || contract.recipe?.groupMethods !== "micom,mga_permutation"
    || contract.recipe?.groupPermutationSamples !== String(mgaRuntimePermutationSamples)
    || contract.recipe?.configuralConfirmed !== "true"
    || contract.recipe?.groupColumn !== "group" || contract.recipe?.groupA !== "A" || contract.recipe?.groupB !== "B"
    || contract.recipe?.method !== "mga" || contract.recipe?.weighting !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.confidenceLevel !== 0.95
    || contract.run?.method !== "MICOM and Two-Group Permutation MGA" || contract.run?.status !== "completed"
    || !Number.isInteger(contract.run?.logs) || contract.run.logs < 1) {
    throw new Error(`The saved group-analysis archive did not retain the exact current MICOM v2 and permutation MGA v2 contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedCcaArchive(projectPath, runId) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved CCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const cca = result.payload?.estimation?.cca;
  const correlations = Array.isArray(cca?.correlations) ? cca.correlations : [];
  const pairKeys = correlations.map((row) => [row.left, row.right].sort().join("::"));
  const finiteRows = correlations.every((row) => [row.observed, row.reproduced, row.residual, row.absolute_residual].every(Number.isFinite));
  const residualIdentities = correlations.every((row) => Math.abs(row.residual - (row.observed - row.reproduced)) <= 1e-10
    && Math.abs(row.absolute_residual - Math.abs(row.residual)) <= 1e-10);
  const calculatedMaximum = Math.max(...correlations.map((row) => row.absolute_residual));
  const contract = {
    resultId: result.id,
    resultStatus: result.status,
    method: result.provenance?.method ?? null,
    methodVersion: result.provenance?.method_version ?? null,
    nestedModelVersion: cca?.model ?? null,
    payloadMethodVersion: cca?.method_version ?? null,
    correlationPairs: correlations.length,
    uniqueCorrelationPairs: new Set(pairKeys).size,
    finiteRows,
    residualIdentities,
    maximumAbsoluteResidual: cca?.max_absolute_residual ?? null,
    maximumMatchesRows: Number.isFinite(cca?.max_absolute_residual)
      && Number.isFinite(calculatedMaximum)
      && Math.abs(cca.max_absolute_residual - calculatedMaximum) <= 1e-10,
    recipe: recipe ? {
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
  };
  if (contract.resultStatus !== "completed"
    || contract.method !== "cca"
    || contract.methodVersion !== ccaProvenanceMethodVersion
    || contract.payloadMethodVersion !== ccaMethodVersion
    || contract.nestedModelVersion !== ccaNestedModelVersion
    || contract.correlationPairs !== 3
    || contract.uniqueCorrelationPairs !== 3
    || !contract.finiteRows
    || !contract.residualIdentities
    || !contract.maximumMatchesRows
    || contract.recipe?.method !== "cca"
    || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized"
    || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0
    || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null
    || contract.recipe?.constructs !== 3
    || contract.recipe?.paths !== 2
    || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0
    || contract.recipe?.higherOrderConstructs !== 0) {
    throw new Error(`The saved CCA archive did not retain the exact bounded recipe and residual payload: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedIpmaArchive(projectPath, runId, constructIds) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved IPMA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const estimation = result.payload?.estimation;
  const ipma = estimation?.ipma;
  const constructRows = Array.isArray(ipma?.constructs) ? ipma.constructs : [];
  const indicatorRows = Array.isArray(ipma?.indicators) ? ipma.indicators : [];
  const expectedPredecessors = [constructIds.x, constructIds.z, constructIds.m].sort();
  const actualPredecessors = constructRows.map((row) => row.construct).sort();
  const actualIndicatorConstructs = indicatorRows.map((row) => row.construct).sort();
  const excludedConstructIds = [constructIds.y, constructIds.u, constructIds.v];
  const excludedIndicators = ["y1", "u1", "v1"];
  const finiteConstructRows = constructRows.every((row) => [row.importance, row.performance, row.score_mean].every(Number.isFinite)
    && row.performance >= 0 && row.performance <= 100 && row.target === constructIds.y);
  const finiteIndicatorRows = indicatorRows.every((row) => [row.construct_importance, row.loading, row.performance, row.score_mean].every(Number.isFinite)
    && row.performance >= 0 && row.performance <= 100 && row.target === constructIds.y);
  const forbiddenNestedPayloads = ["plsc", "wpls", "cca", "predict", "mga", "micom", "mga_permutation", "cbsem", "regression", "nca", "gsca"]
    .filter((key) => estimation?.[key] != null);
  const contract = {
    resultId: result.id,
    resultStatus: result.status,
    method: result.provenance?.method ?? null,
    methodVersion: result.provenance?.method_version ?? null,
    payloadMethodVersion: estimation?.method_version ?? null,
    ipmaMethodVersion: ipma?.method_version ?? null,
    performanceScale: ipma?.performance_scale ?? null,
    targets: ipma?.targets ?? null,
    constructRows: constructRows.length,
    indicatorRows: indicatorRows.length,
    expectedPredecessors,
    actualPredecessors,
    actualIndicatorConstructs,
    indicators: indicatorRows.map((row) => row.indicator).sort(),
    finiteConstructRows,
    finiteIndicatorRows,
    excludedConstructIds,
    excludedIndicators,
    excludesTargetAndUnrelatedRows: excludedConstructIds.every((id) => !actualPredecessors.includes(id)
      && !actualIndicatorConstructs.includes(id))
      && excludedIndicators.every((indicator) => !indicatorRows.some((row) => row.indicator === indicator)),
    forbiddenNestedPayloads,
    recipe: recipe ? {
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      ipmaTargets: recipe.metadata?.ipma_targets ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
  };
  if (contract.resultStatus !== "completed"
    || contract.method !== "ipma"
    || contract.methodVersion !== ipmaProvenanceMethodVersion
    || contract.payloadMethodVersion !== ipmaMethodVersion
    || contract.ipmaMethodVersion !== ipmaMethodVersion
    || contract.performanceScale !== ipmaPerformanceScale
    || JSON.stringify(contract.targets) !== JSON.stringify([constructIds.y])
    || contract.constructRows !== 3
    || contract.indicatorRows !== 3
    || JSON.stringify(actualPredecessors) !== JSON.stringify(expectedPredecessors)
    || JSON.stringify(actualIndicatorConstructs) !== JSON.stringify(expectedPredecessors)
    || JSON.stringify(contract.indicators) !== JSON.stringify(["m1", "x1", "z1"])
    || !finiteConstructRows || !finiteIndicatorRows || !contract.excludesTargetAndUnrelatedRows
    || forbiddenNestedPayloads.length !== 0
    || contract.recipe?.method !== "ipma"
    || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized"
    || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0
    || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null
    || contract.recipe?.ipmaTargets !== constructIds.y
    || contract.recipe?.constructs !== 6
    || contract.recipe?.paths !== 6
    || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0
    || contract.recipe?.higherOrderConstructs !== 0) {
    throw new Error(`The saved IPMA archive did not retain the exact bounded recipe and predecessor-only payload: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function ncaNumberClose(actual, expected) {
  return Number.isFinite(actual) && Math.abs(actual - expected) <= ncaTolerance;
}

function ncaOptionalNumberClose(actual, expected) {
  return expected === null ? actual === null : ncaNumberClose(actual, expected);
}

async function readNcaArchive(projectPath) {
  const [{ stdout: projectText }, { stdout: manifestText }] = await Promise.all([
    execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
      cwd: root,
      windowsHide: true,
      maxBuffer: 32 * 1024 * 1024,
    }),
    execFileAsync("tar", ["-xOf", projectPath, "manifest.json"], {
      cwd: root,
      windowsHide: true,
      maxBuffer: 4 * 1024 * 1024,
    }),
  ]);
  return { project: JSON.parse(projectText), manifest: JSON.parse(manifestText) };
}

async function inspectInitialNcaArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported NCA fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedNcaArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved NCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const nca = estimation?.nca;
  const assessment = result.payload?.assessment;
  const ceilings = Array.isArray(nca?.ceilings) ? nca.ceilings : [];
  const bottlenecks = Array.isArray(nca?.bottlenecks) ? nca.bottlenecks : [];
  const peers = Array.isArray(nca?.ce_fdh_peers) ? nca.ce_fdh_peers : [];
  const scope = nca?.scope;
  const ce = ceilings.find((row) => row.ceiling === "ce_fdh");
  const cr = ceilings.find((row) => row.ceiling === "cr_fdh");
  const expectedPeers = [{ x: 0, y: 1 }, { x: 1, y: 3 }, { x: 3, y: 4 }];
  const expectedCeBottlenecks = [
    100 / 3, 100 / 3, 100 / 3, 100 / 3, 100 / 3, 100 / 3, 100, 100, 100,
  ];
  const expectedCrBottlenecks = [
    null, 6.153846153846154, 16.923076923076923, 27.692307692307693,
    38.46153846153846, 49.23076923076923, 60, 70.76923076923077, 81.53846153846153,
  ];
  const ceBottlenecks = bottlenecks.filter((row) => row.ceiling === "ce_fdh");
  const crBottlenecks = bottlenecks.filter((row) => row.ceiling === "cr_fdh");
  const pValues = ceilings.map((row) => row.permutation_p_value);
  const pValueLattice = pValues.every((value) => Number.isFinite(value)
    && value >= 1 / (ncaPermutationSamples + 1) - ncaTolerance
    && value <= 1 + ncaTolerance
    && Math.abs(value * (ncaPermutationSamples + 1) - Math.round(value * (ncaPermutationSamples + 1))) <= ncaTolerance);
  const scopeMatches = scope
    && ncaNumberClose(scope.minimum_x, 0) && ncaNumberClose(scope.maximum_x, 3)
    && ncaNumberClose(scope.minimum_y, 1) && ncaNumberClose(scope.maximum_y, 4);
  const peersMatch = peers.length === expectedPeers.length && peers.every((peer, index) => (
    ncaNumberClose(peer.x, expectedPeers[index].x) && ncaNumberClose(peer.y, expectedPeers[index].y)
  ));
  const ceilingGeometryMatches = ceilings.length === 2
    && ce && cr
    && ncaNumberClose(ce.effect_size, 5 / 9) && ce.slope === null && ce.intercept === null
    && ncaNumberClose(cr.effect_size, 36 / 91)
    && ncaNumberClose(cr.slope, 13 / 14) && ncaNumberClose(cr.intercept, 10 / 7)
    && pValueLattice;
  const bottlenecksMatch = bottlenecks.length === 18
    && ceBottlenecks.length === 9 && crBottlenecks.length === 9
    && ceBottlenecks.every((row, index) => row.outcome_percent === (index + 1) * 10
      && row.status === "required"
      && ncaOptionalNumberClose(row.required_x_percent, expectedCeBottlenecks[index]))
    && crBottlenecks.every((row, index) => row.outcome_percent === (index + 1) * 10
      && row.status === (index === 0 ? "not_necessary" : "required")
      && ncaOptionalNumberClose(row.required_x_percent, expectedCrBottlenecks[index]));
  const exactNcaKeys = Object.keys(nca ?? {}).sort();
  const expectedNcaKeys = [
    "bottlenecks", "ce_fdh_peers", "ceiling", "ceilings", "method_version", "observations",
    "permutation_samples", "scope", "usable_permutations", "warnings", "x", "y",
  ].sort();
  const expectedCeilingKeys = ["ceiling", "effect_size", "permutation_p_value", "slope", "intercept"].sort();
  const expectedBottleneckKeys = ["ceiling", "outcome_percent", "required_x_percent", "status"].sort();
  const expectedPeerKeys = ["x", "y"].sort();
  const exactCeilingRows = ceilings.every((row) => (
    JSON.stringify(Object.keys(row).sort()) === JSON.stringify(expectedCeilingKeys)
  ));
  const exactBottleneckRows = bottlenecks.every((row) => (
    JSON.stringify(Object.keys(row).sort()) === JSON.stringify(expectedBottleneckKeys)
  ));
  const exactPeerRows = peers.every((row) => (
    JSON.stringify(Object.keys(row).sort()) === JSON.stringify(expectedPeerKeys)
  ));
  const forbiddenNestedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nonlinear_effects", "pca", "plsc", "predict", "regression",
    "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const contract = {
    resultId: result.id,
    resultStatus: result.status,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    provenanceSeed: result.provenance?.seed ?? null,
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? {
      methodVersion: assessment.method_version ?? null,
      warnings: assessment.warnings ?? null,
    } : null,
    ncaKeys: exactNcaKeys,
    exactCeilingRows,
    exactBottleneckRows,
    exactPeerRows,
    ncaMethodVersion: nca?.method_version ?? null,
    ceiling: nca?.ceiling ?? null,
    permutationSamples: nca?.permutation_samples ?? null,
    usablePermutations: nca?.usable_permutations ?? null,
    x: nca?.x ?? null,
    y: nca?.y ?? null,
    observations: nca?.observations ?? null,
    scope,
    peers,
    ceilings,
    bottlenecks,
    scopeMatches,
    peersMatch,
    ceilingGeometryMatches,
    bottlenecksMatch,
    pValueLattice,
    warnings: nca?.warnings ?? null,
    forbiddenNestedPayloads,
    recipe: recipe ? {
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      seed: recipe.settings?.seed ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      ncaX: recipe.metadata?.nca_x ?? null,
      ncaY: recipe.metadata?.nca_y ?? null,
      ncaCeiling: recipe.metadata?.nca_ceiling ?? null,
      ncaPermutationSamples: recipe.metadata?.nca_permutation_samples ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runModelSnapshot: run?.modelSnapshot ?? null,
    runSeed: run?.seed ?? null,
  };
  if (contract.resultStatus !== "completed"
    || contract.provenanceMethod !== "nca"
    || contract.provenanceMethodVersion !== ncaMethodVersion
    || contract.provenanceEngineVersion !== packageVersion
    || contract.provenanceSeed !== ncaSeed
    || contract.manifestEngineVersion !== packageVersion
    || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== ncaMethodVersion
    || contract.usedObservations !== 4 || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || JSON.stringify(exactNcaKeys) !== JSON.stringify(expectedNcaKeys)
    || !exactCeilingRows || !exactBottleneckRows || !exactPeerRows
    || contract.ncaMethodVersion !== ncaMethodVersion || contract.ceiling !== "both"
    || contract.permutationSamples !== ncaPermutationSamples || contract.usablePermutations !== ncaPermutationSamples
    || contract.x !== "x" || contract.y !== "y" || contract.observations !== 4
    || !scopeMatches || !peersMatch || !ceilingGeometryMatches || !bottlenecksMatch
    || !Array.isArray(contract.warnings) || contract.warnings.length !== 1
    || !/numeric X\/Y CE-FDH and CR-FDH scope with observed-range bottlenecks/i.test(contract.warnings[0])
    || forbiddenNestedPayloads.length !== 0
    || contract.recipe?.method !== "nca" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "unstandardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.seed !== ncaSeed || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.ncaX !== "x" || contract.recipe?.ncaY !== "y"
    || contract.recipe?.ncaCeiling !== "both" || contract.recipe?.ncaPermutationSamples !== String(ncaPermutationSamples)
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0 || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || contract.models !== 0 || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0
    || contract.runModelId !== null || contract.runModelSnapshot !== null
    || contract.runSeed !== ncaSeed) {
    throw new Error(`The saved NCA archive did not retain the exact standalone nca_v2 recipe, geometry, and model-free snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectInitialPcaArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported PCA fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedPcaArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved PCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const pca = estimation?.pca;
  const components = Array.isArray(pca?.components) ? pca.components : [];
  const loadings = Array.isArray(pca?.loadings) ? pca.loadings : [];
  const scores = Array.isArray(pca?.scores) ? pca.scores : [];
  const assessment = result.payload?.assessment;
  const componentIds = components.map((row) => row.component);
  const expectedComponentIds = ["PC1", "PC2", "PC3", "PC4"];
  const uniqueLoadingIdentities = new Set(loadings.map((row) => `${row.variable}\u0000${row.component}`));
  const uniqueScoreIdentities = new Set(scores.map((row) => `${row.observation}\u0000${row.component}`));
  const finiteComponents = components.every((row) => [row.eigenvalue, row.explained_variance, row.cumulative_variance].every(Number.isFinite));
  const finiteLoadings = loadings.every((row) => Number.isFinite(row.loading) && Number.isFinite(row.weight));
  const finiteScores = scores.every((row) => Number.isInteger(row.observation) && Number.isFinite(row.score));
  const thresholdCrossing = components.length === 4
    && components[2].cumulative_variance < pcaVarianceThreshold
    && components[3].cumulative_variance >= pcaVarianceThreshold;
  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nca", "nonlinear_effects", "plsc", "predict", "regression", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    pcaMethodVersion: pca?.method_version ?? null,
    componentRule: pca?.component_rule ?? null,
    retainedComponents: pca?.retained_components ?? null,
    observations: pca?.observations ?? null,
    variables: pca?.variables ?? null,
    componentIds,
    components,
    loadings: loadings.length,
    scores: scores.length,
    uniqueLoadingIdentities: uniqueLoadingIdentities.size,
    uniqueScoreIdentities: uniqueScoreIdentities.size,
    finiteComponents,
    finiteLoadings,
    finiteScores,
    thresholdCrossing,
    warnings: pca?.warnings ?? null,
    unrelatedPayloads,
    recipe: recipe ? {
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      variables: recipe.metadata?.pca_variables ?? null,
      componentRule: recipe.metadata?.pca_component_rule ?? null,
      varianceThreshold: recipe.metadata?.pca_variance_threshold ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runModelSnapshot: run?.modelSnapshot ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "pca"
    || contract.provenanceMethodVersion !== pcaMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== pcaMethodVersion || contract.pcaMethodVersion !== pcaMethodVersion
    || contract.usedObservations !== 140 || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || contract.componentRule !== "variance_threshold" || contract.retainedComponents !== 4
    || contract.observations !== 140 || JSON.stringify(contract.variables) !== JSON.stringify(pcaVariables)
    || JSON.stringify(componentIds) !== JSON.stringify(expectedComponentIds)
    || contract.loadings !== 20 || contract.scores !== 560
    || contract.uniqueLoadingIdentities !== 20 || contract.uniqueScoreIdentities !== 560
    || !finiteComponents || !finiteLoadings || !finiteScores || !thresholdCrossing
    || !Array.isArray(contract.warnings) || contract.warnings.length !== 1
    || !/Standalone PCA v1 is validated/i.test(contract.warnings[0]) || unrelatedPayloads.length !== 0
    || contract.recipe?.method !== "pca" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.variables !== pcaVariables.join(",")
    || contract.recipe?.componentRule !== "variance_threshold"
    || Number(contract.recipe?.varianceThreshold) !== pcaVarianceThreshold
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0
    || contract.models !== 0 || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0
    || contract.runModelId !== null || contract.runModelSnapshot !== null) {
    throw new Error(`The saved PCA archive did not retain the exact standalone pca_v1 variance-threshold result and model-free snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedHigherOrderArchive(projectPath, runId, expected) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved HOC archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  if (!recipe || !run) throw new Error("The saved HOC archive did not retain its canonical recipe and visible run snapshot.");
  const declaration = recipe.model?.higher_order_constructs?.[0];
  const hocConstruct = recipe.model?.constructs?.find((construct) => construct.id === expected.hocId);
  const pathRow = recipe.model?.paths?.[0];
  const estimation = result.payload?.estimation;
  const expectedGeneratedIndicators = expected.componentIds
    .map((componentId) => `__qpls_hoc_${expected.hocId}_${componentId}`)
    .sort();
  const generatedOuterRows = (estimation?.outer_estimates ?? [])
    .filter((row) => row.construct === expected.hocId)
    .sort((left, right) => String(left.indicator).localeCompare(String(right.indicator)));
  const hocSnapshotNode = run.modelSnapshot?.nodes?.find((node) => node.id === expected.hocId);
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    recipeId: recipe.id ?? null,
    recipeMethod: recipe.settings?.method ?? null,
    weightingScheme: recipe.settings?.weighting_scheme ?? null,
    preprocessing: recipe.settings?.preprocessing ?? null,
    missingData: recipe.settings?.missing_data ?? null,
    bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
    studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
    permutationSamples: recipe.settings?.permutation_samples ?? null,
    caseWeightColumn: recipe.settings?.case_weight_column ?? null,
    constructCount: recipe.model?.constructs?.length ?? null,
    pathCount: recipe.model?.paths?.length ?? null,
    higherOrderCount: recipe.model?.higher_order_constructs?.length ?? null,
    declaration: declaration ? {
      id: declaration.id,
      components: declaration.components,
      method: declaration.method,
      stageOneRecipe: declaration.stage_one_recipe,
    } : null,
    hocConstruct: hocConstruct ? {
      mode: hocConstruct.mode,
      indicators: hocConstruct.indicators,
    } : null,
    path: pathRow ? { source: pathRow.source, target: pathRow.target } : null,
    generatedOuterIndicators: generatedOuterRows.map((row) => row.indicator),
    finiteGeneratedOuterRows: generatedOuterRows.every((row) => Number.isFinite(row.loading) && Number.isFinite(row.weight)),
    generatedTransformKeys: (estimation?.transforms ?? [])
      .map((row) => row.indicator)
      .filter((indicator) => String(indicator).startsWith("__qpls_hoc_"))
      .sort(),
    constructScoreIds: Object.keys(estimation?.construct_scores ?? {}).sort(),
    runModelId: run.modelId ?? null,
    runSnapshot: hocSnapshotNode ? {
      semantic: hocSnapshotNode.data?.semantic ?? null,
      indicators: hocSnapshotNode.data?.indicators ?? null,
      declaration: hocSnapshotNode.data?.higherOrder ?? null,
    } : null,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.resultStatus !== "completed"
    || contract.provenanceMethod !== "pls_pm"
    || !String(contract.provenanceMethodVersion).includes("pls_pm_v1")
    || contract.payloadKind !== "pls_pm_v1"
    || contract.recipeMethod !== "pls_pm"
    || contract.weightingScheme !== "path"
    || contract.preprocessing !== "standardized"
    || contract.missingData !== "listwise_deletion"
    || contract.bootstrapSamples !== 0 || contract.studentizedInnerSamples !== 0 || contract.permutationSamples !== 0
    || contract.caseWeightColumn !== null
    || contract.constructCount !== 4 || contract.pathCount !== 1 || contract.higherOrderCount !== 1
    || contract.declaration?.id !== expected.hocId
    || JSON.stringify([...(contract.declaration?.components ?? [])].sort()) !== JSON.stringify([...expected.componentIds].sort())
    || contract.declaration?.method !== "two_stage" || contract.declaration?.stageOneRecipe !== null
    || contract.hocConstruct?.mode !== "reflective" || contract.hocConstruct?.indicators?.length !== 0
    || contract.path?.source !== expected.hocId || contract.path?.target !== expected.outcomeId
    || JSON.stringify(contract.generatedOuterIndicators) !== JSON.stringify(expectedGeneratedIndicators)
    || !contract.finiteGeneratedOuterRows
    || JSON.stringify(contract.generatedTransformKeys) !== JSON.stringify(expectedGeneratedIndicators)
    || !expected.componentIds.every((componentId) => contract.constructScoreIds.includes(componentId))
    || !contract.constructScoreIds.includes(expected.hocId) || !contract.constructScoreIds.includes(expected.outcomeId)
    || !contract.runModelId
    || contract.runSnapshot?.semantic !== "higher_order"
    || contract.runSnapshot?.indicators?.length !== 0
    || contract.runSnapshot?.declaration?.method !== "two_stage") {
    throw new Error(`The saved HOC archive did not retain the exact bounded two-stage model, generated identities, and completed result: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectXlsxWorkbookSheets(filePath) {
  const { stdout } = await execFileAsync("tar", ["-xOf", filePath, "xl/workbook.xml"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 4 * 1024 * 1024,
  });
  return [...stdout.matchAll(/<sheet\s+name="([^"]+)"/g)].map((match) => match[1]);
}

function mediationCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-${state}-${nativeViewportLabel}.png`;
}

function moderationCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-${state}-${nativeViewportLabel}.png`;
}

function mgaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-mga-${state}-${nativeViewportLabel}.png`;
}

function ccaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-cca-${state}-${nativeViewportLabel}.png`;
}

function ipmaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-ipma-${state}-${nativeViewportLabel}.png`;
}

function ncaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-nca-${state}-${nativeViewportLabel}.png`;
}

function predictionCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-prediction-${state}-${nativeViewportLabel}.png`;
}

function hocCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-hoc-${state}-${nativeViewportLabel}.png`;
}

async function inspectInitialOlsArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported OLS fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedOlsArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved OLS archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const regression = estimation?.regression;
  const coefficients = Array.isArray(regression?.coefficients) ? regression.coefficients : [];
  const predictions = Array.isArray(regression?.predictions) ? regression.predictions : [];
  const assessment = result.payload?.assessment;
  const expectedTerms = ["intercept", ...olsPredictors, ...olsControls];
  const coefficientContract = coefficients.length === expectedTerms.length && coefficients.every((row, index) => (
    row.term === expectedTerms[index]
    && [row.estimate, row.standard_error, row.statistic, row.p_value_two_sided, row.confidence_interval_lower, row.confidence_interval_upper].every(Number.isFinite)
    && row.standard_error > 0 && row.p_value_two_sided >= 0 && row.p_value_two_sided <= 1
    && row.confidence_interval_lower <= row.estimate && row.confidence_interval_upper >= row.estimate
    && row.odds_ratio == null
  ));
  const predictionContract = predictions.length === 140 && predictions.every((row, index) => (
    row.observation === index && Number.isFinite(row.fitted) && Number.isFinite(row.residual) && row.probability == null
  ));
  const fit = regression?.fit;
  const fitContract = fit
    && [fit.r_squared, fit.adjusted_r_squared, fit.f_statistic, fit.aic, fit.bic, fit.rmse].every(Number.isFinite)
    && fit.log_likelihood == null && fit.pseudo_r_squared == null;
  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    regressionMethodVersion: regression?.method_version ?? null,
    regressionType: regression?.regression_type ?? null,
    outcome: regression?.outcome ?? null,
    predictors: regression?.predictors ?? null,
    controls: regression?.controls ?? null,
    observations: regression?.observations ?? null,
    coefficientCount: coefficients.length,
    predictionCount: predictions.length,
    coefficientContract,
    predictionContract,
    fitContract: Boolean(fitContract),
    process: regression?.process ?? null,
    warnings: regression?.warnings ?? null,
    unrelatedPayloads,
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      regressionType: recipe.metadata?.regression_type ?? null,
      outcome: recipe.metadata?.regression_outcome ?? null,
      predictors: recipe.metadata?.regression_predictors ?? null,
      controls: recipe.metadata?.regression_controls ?? null,
      robustSe: recipe.metadata?.robust_se ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controlsCount: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runModelSnapshot: run?.modelSnapshot ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "regression"
    || contract.provenanceMethodVersion !== olsMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== olsMethodVersion || contract.regressionMethodVersion !== olsMethodVersion
    || contract.usedObservations !== 140 || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || contract.regressionType !== "ols" || contract.outcome !== olsOutcome
    || JSON.stringify(contract.predictors) !== JSON.stringify(olsPredictors)
    || JSON.stringify(contract.controls) !== JSON.stringify(olsControls)
    || contract.observations !== 140 || contract.coefficientCount !== 4 || contract.predictionCount !== 140
    || !contract.coefficientContract || !contract.predictionContract || !contract.fitContract || contract.process !== null
    || !Array.isArray(contract.warnings) || contract.warnings.length !== 1 || !/OLS regression v1 is validated/i.test(contract.warnings[0])
    || contract.unrelatedPayloads.length !== 0
    || contract.recipe?.status !== "validated_regression_ols_v1_bounded_scope"
    || contract.recipe?.method !== "regression" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "unstandardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.confidenceLevel !== 0.95 || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.regressionType !== "ols"
    || contract.recipe?.outcome !== olsOutcome || contract.recipe?.predictors !== olsPredictors.join(",")
    || contract.recipe?.controls !== olsControls.join(",") || contract.recipe?.robustSe !== "hc3"
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0 || contract.recipe?.controlsCount !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || contract.models !== 0 || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0
    || contract.runModelId !== null || contract.runModelSnapshot !== null) {
    throw new Error(`The saved OLS archive did not retain the exact standalone regression_ols_v1 result and model-free snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectInitialCbsemArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported CB-SEM fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedCbsemArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved CB-SEM archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const cbsem = estimation?.cbsem;
  const parameters = Array.isArray(cbsem?.parameters) ? cbsem.parameters : [];
  const standardized = Array.isArray(cbsem?.standardized) ? cbsem.standardized : [];
  const implied = Array.isArray(cbsem?.implied_covariance) ? cbsem.implied_covariance : [];
  const residualCovariance = Array.isArray(cbsem?.residual_covariance) ? cbsem.residual_covariance : [];
  const residualCorrelation = Array.isArray(cbsem?.residual_correlation) ? cbsem.residual_correlation : [];
  const modification = Array.isArray(cbsem?.modification_indices) ? cbsem.modification_indices : [];
  const parameterNames = parameters.map((row) => row.name);
  const standardizedNames = standardized.map((row) => row.name);
  const matrixContract = (rows) => rows.length === 81 && rows.every((row) => (
    typeof row.row === "string" && row.row.length > 0
    && typeof row.column === "string" && row.column.length > 0
    && Number.isFinite(row.value)
  ));
  const fit = cbsem?.fit;
  const fitContract = fit
    && fit.method_version === cbsemFitMethodVersion
    && [fit.chi_square, fit.srmr, fit.aic, fit.bic, fit.baseline_chi_square].every(Number.isFinite)
    && Number.isInteger(fit.degrees_of_freedom) && fit.degrees_of_freedom >= 0
    && Number.isInteger(fit.baseline_degrees_of_freedom) && fit.baseline_degrees_of_freedom >= 0
    && [fit.p_value, fit.cfi, fit.tli, fit.rmsea, fit.rmsea_ci_lower, fit.rmsea_ci_upper].every((value) => value === null || Number.isFinite(value));
  const unrelatedPayloads = [
    "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation", "micom",
    "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "regression", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const constructIndicators = recipe?.model?.constructs?.map((construct) => ({
    name: construct.name,
    mode: construct.mode,
    indicators: construct.indicators?.length ?? 0,
  })) ?? [];
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    cbsem: cbsem ? {
      methodVersion: cbsem.method_version ?? null,
      modelType: cbsem.model_type ?? null,
      estimator: cbsem.estimator ?? null,
      input: cbsem.input ?? null,
      meanStructure: cbsem.mean_structure ?? null,
      converged: cbsem.converged ?? null,
      iterations: cbsem.iterations ?? null,
      objective: cbsem.objective ?? null,
      gradientNorm: cbsem.gradient_norm ?? null,
      sampleSize: cbsem.sample_size ?? null,
      parameterCount: parameters.length,
      uniqueParameterNames: new Set(parameterNames).size,
      standardizedCount: standardized.length,
      standardizedIdentityMatch: JSON.stringify(standardizedNames) === JSON.stringify(parameterNames),
      impliedCount: implied.length,
      residualCovarianceCount: residualCovariance.length,
      residualCorrelationCount: residualCorrelation.length,
      matrixContract: matrixContract(implied) && matrixContract(residualCovariance) && matrixContract(residualCorrelation),
      modificationCount: modification.length,
      modificationContract: modification.length > 0 && modification.every((row) => (
        row.method_version === cbsemModificationMethodVersion
        && typeof row.kind === "string" && typeof row.lhs === "string" && typeof row.rhs === "string"
        && Number.isFinite(row.modification_index)
        && (row.expected_parameter_change === null || Number.isFinite(row.expected_parameter_change))
      )),
      fitContract: Boolean(fitContract),
      diagnostics: cbsem.diagnostics ?? null,
      warnings: cbsem.warnings ?? null,
      bootstrap: cbsem.bootstrap ?? null,
      multigroup: cbsem.multigroup ?? null,
    } : null,
    unrelatedPayloads,
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      workers: recipe.settings?.workers ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      modelType: recipe.metadata?.cbsem_model_type ?? null,
      estimator: recipe.metadata?.cbsem_estimator ?? null,
      input: recipe.metadata?.cbsem_input ?? null,
      meanStructure: recipe.metadata?.cbsem_mean_structure ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      constructIndicators,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runSnapshotNodes: run?.modelSnapshot?.nodes?.length ?? null,
    runSnapshotEdges: run?.modelSnapshot?.edges?.length ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "cbsem"
    || contract.provenanceMethodVersion !== cbsemProvenanceMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== cbsemMethodVersion || contract.usedObservations !== 240 || contract.omittedObservations !== 0
    || !contract.cbsem || contract.cbsem.methodVersion !== cbsemMethodVersion || contract.cbsem.modelType !== "sem"
    || contract.cbsem.estimator !== "ml" || contract.cbsem.input !== "raw" || contract.cbsem.meanStructure !== false
    || contract.cbsem.converged !== true || !Number.isInteger(contract.cbsem.iterations) || contract.cbsem.iterations < 1
    || !Number.isFinite(contract.cbsem.objective) || !Number.isFinite(contract.cbsem.gradientNorm) || contract.cbsem.sampleSize !== 240
    || contract.cbsem.parameterCount !== 23 || contract.cbsem.uniqueParameterNames !== 23
    || contract.cbsem.standardizedCount !== 23 || !contract.cbsem.standardizedIdentityMatch
    || contract.cbsem.impliedCount !== 81 || contract.cbsem.residualCovarianceCount !== 81 || contract.cbsem.residualCorrelationCount !== 81
    || !contract.cbsem.matrixContract || contract.cbsem.modificationCount !== 50 || !contract.cbsem.modificationContract
    || !contract.cbsem.fitContract || !Array.isArray(contract.cbsem.diagnostics) || !Array.isArray(contract.cbsem.warnings)
    || contract.cbsem.bootstrap !== null || contract.cbsem.multigroup !== null || contract.unrelatedPayloads.length !== 0
    || contract.recipe?.status !== "validated_v1_2_4_cbsem_single_group_bounded_scope"
    || contract.recipe?.method !== "cbsem" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.workers !== 1 || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.modelType !== "sem"
    || contract.recipe?.estimator !== "ml" || contract.recipe?.input !== "raw" || contract.recipe?.meanStructure !== "false"
    || contract.recipe?.constructs !== 3 || contract.recipe?.paths !== 2 || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || constructIndicators.length !== 3 || constructIndicators.some((construct) => construct.mode !== "reflective" || construct.indicators !== 3)
    || contract.models !== 1 || !contract.activeModelId || contract.nodes !== 3 || contract.edges !== 2
    || contract.runModelId !== contract.activeModelId || contract.runSnapshotNodes !== 3 || contract.runSnapshotEdges !== 2) {
    throw new Error(`The saved CB-SEM archive did not retain the exact bounded ML recipe, payload, and model snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectInitialGscaArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported GSCA fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function gscaRowsMatch(actualRows, expectedRows, identity) {
  if (!Array.isArray(actualRows) || !Array.isArray(expectedRows) || actualRows.length !== expectedRows.length) return false;
  const expectedById = new Map(expectedRows.map((row) => [identity(row), row]));
  return actualRows.every((row) => {
    const expected = expectedById.get(identity(row));
    if (!expected) return false;
    return Object.keys(expected).every((key) => typeof expected[key] === "number"
      ? ncaNumberClose(row[key], expected[key])
      : row[key] === expected[key]);
  });
}

async function inspectSavedGscaArchive(projectPath, runId) {
  const [{ project, manifest }, referenceEnvelope] = await Promise.all([
    readNcaArchive(projectPath),
    fs.readFile(gscaReferenceOutputPath, "utf8").then(JSON.parse),
  ]);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved GSCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const gsca = estimation?.gsca;
  const expected = referenceEnvelope.payload?.estimation?.gsca;
  const assessment = result.payload?.assessment;
  const gscaKeys = Object.keys(gsca ?? {}).sort();
  const expectedGscaKeys = [
    "adjusted_fit", "algorithm", "bootstrap_intervals", "converged", "covariance_discrepancy",
    "covariance_sample_total", "final_change", "fit", "free_parameters", "gfi", "iterations",
    "loadings", "measurement_fit", "method_version", "objective", "observations", "paths",
    "r_squared", "srmr", "standardized_residual_sum", "stop_criterion", "structural_fit", "warnings", "weights",
  ].sort();
  const metricKeys = [
    "objective", "fit", "measurement_fit", "structural_fit", "adjusted_fit", "gfi", "srmr",
    "covariance_discrepancy", "covariance_sample_total", "standardized_residual_sum", "final_change",
  ];
  const metricsMatch = Boolean(gsca && expected && metricKeys.every((key) => ncaNumberClose(gsca[key], expected[key])));
  const gConstructId = recipe?.model?.constructs?.find((construct) => (
    construct.mode === "formative" && JSON.stringify(construct.indicators ?? []) === JSON.stringify(["g1", "g2", "g3"])
  ))?.id;
  const hConstructId = recipe?.model?.constructs?.find((construct) => (
    construct.mode === "reflective" && JSON.stringify(construct.indicators ?? []) === JSON.stringify(["h1", "h2"])
  ))?.id;
  const expectedConstructId = (referenceId) => referenceId === "g" ? gConstructId : referenceId === "h" ? hConstructId : referenceId;
  const remapExpectedRows = (rows, fields) => Array.isArray(rows) ? rows.map((row) => ({
    ...row,
    ...Object.fromEntries(fields.map((field) => [field, expectedConstructId(row[field])])),
  })) : rows;
  const expectedWeights = remapExpectedRows(expected?.weights, ["construct"]);
  const expectedLoadings = remapExpectedRows(expected?.loadings, ["construct"]);
  const expectedPaths = remapExpectedRows(expected?.paths, ["source", "target"]);
  const expectedRSquared = expected && Object.fromEntries(Object.entries(expected.r_squared ?? {})
    .map(([construct, value]) => [expectedConstructId(construct), value]));
  const weightsMatch = gscaRowsMatch(gsca?.weights, expectedWeights, (row) => `${row.construct}\u0000${row.indicator}`);
  const loadingsMatch = gscaRowsMatch(gsca?.loadings, expectedLoadings, (row) => `${row.construct}\u0000${row.indicator}`);
  const pathsMatch = gscaRowsMatch(gsca?.paths, expectedPaths, (row) => `${row.source}\u0000${row.target}`);
  const rSquaredMatch = gsca && expectedRSquared
    && JSON.stringify(Object.keys(gsca.r_squared ?? {}).sort()) === JSON.stringify(Object.keys(expectedRSquared).sort())
    && Object.keys(expectedRSquared).every((key) => ncaNumberClose(gsca.r_squared[key], expectedRSquared[key]));
  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "ipma", "mga", "mga_permutation", "micom",
    "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "regression", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const constructIndicators = recipe?.model?.constructs?.map((construct) => ({
    name: construct.name,
    mode: construct.mode,
    indicators: construct.indicators ?? [],
  })) ?? [];
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    converged: estimation?.converged ?? null,
    iterations: estimation?.iterations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    gsca: gsca ? {
      keys: gscaKeys,
      methodVersion: gsca.method_version ?? null,
      algorithm: gsca.algorithm ?? null,
      converged: gsca.converged ?? null,
      iterations: gsca.iterations ?? null,
      stopCriterion: gsca.stop_criterion ?? null,
      finalChange: gsca.final_change ?? null,
      observations: gsca.observations ?? null,
      freeParameters: gsca.free_parameters ?? null,
      metricsMatch,
      weightsMatch,
      loadingsMatch,
      pathsMatch,
      rSquaredMatch,
      bootstrapIntervals: gsca.bootstrap_intervals ?? null,
      warnings: gsca.warnings ?? null,
    } : null,
    unrelatedPayloads,
    noPlsArtifacts: Array.isArray(estimation?.effects) && estimation.effects.length === 0
      && Array.isArray(estimation?.control_estimates) && estimation.control_estimates.length === 0
      && (estimation?.mediation?.estimates?.length ?? -1) === 0
      && (estimation?.moderation?.estimates?.length ?? -1) === 0,
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      maxIterations: recipe.settings?.max_iterations ?? null,
      tolerance: recipe.settings?.tolerance ?? null,
      workers: recipe.settings?.workers ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      constructIndicators,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runSnapshotNodes: run?.modelSnapshot?.nodes?.length ?? null,
    runSnapshotEdges: run?.modelSnapshot?.edges?.length ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "gsca"
    || contract.provenanceMethodVersion !== gscaMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== gscaMethodVersion || contract.usedObservations !== 140 || contract.omittedObservations !== 0
    || contract.converged !== true || contract.iterations !== 4
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to GSCA ALS component-model estimation."])
    || !contract.gsca || JSON.stringify(contract.gsca.keys) !== JSON.stringify(expectedGscaKeys)
    || contract.gsca.methodVersion !== gscaMethodVersion || contract.gsca.algorithm !== gscaAlgorithmVersion
    || contract.gsca.converged !== true || contract.gsca.iterations !== 4 || contract.gsca.stopCriterion !== 1e-7
    || !Number.isFinite(contract.gsca.finalChange) || contract.gsca.finalChange > 1e-7
    || contract.gsca.observations !== 140 || contract.gsca.freeParameters !== 6
    || !contract.gsca.metricsMatch || !contract.gsca.weightsMatch || !contract.gsca.loadingsMatch
    || !contract.gsca.pathsMatch || !contract.gsca.rSquaredMatch
    || !Array.isArray(contract.gsca.bootstrapIntervals) || contract.gsca.bootstrapIntervals.length !== 0
    || !Array.isArray(contract.gsca.warnings) || contract.gsca.warnings.length !== 1
    || !/GSCA ALS v2 is bounded/i.test(contract.gsca.warnings[0])
    || contract.unrelatedPayloads.length !== 0 || !contract.noPlsArtifacts
    || contract.recipe?.status !== "validated_gsca_als_v2_bounded_scope"
    || contract.recipe?.method !== "gsca" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.maxIterations !== 3_000 || contract.recipe?.tolerance !== 1e-7 || contract.recipe?.workers !== 1
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.studentizedInnerSamples !== 0
    || contract.recipe?.permutationSamples !== 0 || contract.recipe?.caseWeightColumn !== null
    || contract.recipe?.constructs !== 2 || contract.recipe?.paths !== 1 || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || JSON.stringify(constructIndicators) !== JSON.stringify([
      { name: "G formative component", mode: "formative", indicators: ["g1", "g2", "g3"] },
      { name: "H reflective component", mode: "reflective", indicators: ["h1", "h2"] },
    ])
    || contract.models !== 1 || !contract.activeModelId || contract.nodes !== 2 || contract.edges !== 1
    || contract.runModelId !== contract.activeModelId || contract.runSnapshotNodes !== 2 || contract.runSnapshotEdges !== 1) {
    throw new Error(`The saved GSCA archive did not retain the exact ALS v2 recipe, numerical payload, and model snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function olsCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-ols-${state}-${nativeViewportLabel}.png`;
}

function pcaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-pca-${state}-${nativeViewportLabel}.png`;
}

function cbsemCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-cbsem-${state}-${nativeViewportLabel}.png`;
}

function gscaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-gsca-${state}-${nativeViewportLabel}.png`;
}

async function runFocusedCbsemAcceptance() {
  if (!requestedCbsemNativeExportPath) {
    throw new Error("QUICKPLS_CBSEM_NATIVE_EXPORT_PATH is required for focused packaged CB-SEM acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedCbsemNativeExportPath,
    "QUICKPLS_CBSEM_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: cbsemProjectName, path: cbsemProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(cbsemProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialCbsemArchive(cbsemProjectPath);
  evidence.checks.cbsemFixture = {
    projectPath: cbsemProjectPath,
    sourceCsv: cbsemFixtureCsvPath,
    status,
    cases: status.includes("240 cases") ? 240 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x1", "x2", "x3", "m1", "m2", "m3", "y1", "y2", "y3"];
  if (evidence.checks.cbsemFixture.cases !== 240 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused CB-SEM fixture did not expose the canonical 240-row data-only project: ${JSON.stringify(evidence.checks.cbsemFixture)}`);
  }
  await capture(cbsemCaptureName(130, "fixture-data"));

  evidence.checks.cbsemInitialModelCreation = await createInitialEditableModel(cbsemProjectName, cbsemModelName);
  await buildThreeConstructCbsemModel();
  evidence.checks.cbsemModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map(compactVisibleText),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (evidence.checks.cbsemModel.constructs !== 3 || evidence.checks.cbsemModel.assignedIndicators !== 9
    || evidence.checks.cbsemModel.structuralPaths !== 2
    || !["X", "M", "Y"].every((name) => evidence.checks.cbsemModel.constructLabels.some((label) => label.includes(name)))) {
    throw new Error(`The visible CB-SEM authoring workflow did not create X -> M -> Y with three reflective indicators per factor: ${JSON.stringify(evidence.checks.cbsemModel)}`);
  }
  await capture(cbsemCaptureName(131, "model"));
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const calculation = await openCalculationFromToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-cbsem").click();
  const modelType = calculation.locator("#nd-calculation-cbsem-model-type");
  await modelType.waitFor({ state: "visible", timeout: 10_000 });
  await modelType.selectOption("sem");
  const maximumIterations = calculation.locator("#nd-calculation-max-iterations");
  const tolerance = calculation.locator("#nd-calculation-tolerance");
  const start = calculation.getByRole("button", { name: "Start CB-SEM / CFA", exact: true });
  const blockers = await calculation.locator(".nd-blocker li").allTextContents();
  evidence.checks.cbsemDialog = {
    catalogCount: await options.count(),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-covariance").textContent()),
    modelTypeOptions: await modelType.locator("option").evaluateAll((entries) => entries.map((entry) => ({ value: entry.value, text: entry.textContent?.trim() ?? "" }))),
    modelType: await modelType.inputValue(),
    weighting: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Weighting scheme" }).locator("strong").textContent()),
    resultData: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Result data" }).locator("strong").textContent()),
    editableWeightingControls: await calculation.locator("#nd-calculation-weighting, #nd-calculation-preprocessing").count(),
    maximumIterations: await maximumIterations.inputValue(),
    tolerance: await tolerance.inputValue(),
    estimator: compactVisibleText(await calculation.locator("#nd-calculation-cbsem-estimator").textContent()),
    scope: compactVisibleText(await calculation.locator("#nd-calculation-cbsem-scope").textContent()),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations", "#nd-calculation-nca-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight", "#nd-calculation-group-column",
      "#nd-calculation-cbsem-bootstrap", "#nd-calculation-cbsem-group", "#nd-calculation-cbsem-mean-structure",
    ].join(", ")).count(),
    blockers,
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.cbsemDialog.catalogCount !== 13 || evidence.checks.cbsemDialog.selectedMethod !== "CB-SEM / CFA"
    || evidence.checks.cbsemDialog.category !== "Covariance-based SEM"
    || JSON.stringify(evidence.checks.cbsemDialog.modelTypeOptions) !== JSON.stringify([
      { value: "sem", text: "Structural equation model (paths required)" },
      { value: "cfa", text: "Confirmatory factor analysis (no paths)" },
    ])
    || evidence.checks.cbsemDialog.modelType !== "sem" || evidence.checks.cbsemDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.cbsemDialog.resultData !== "Standardized (fixed)"
    || evidence.checks.cbsemDialog.editableWeightingControls !== 0
    || !Number.isInteger(Number(evidence.checks.cbsemDialog.maximumIterations)) || Number(evidence.checks.cbsemDialog.maximumIterations) < 1
    || !Number.isFinite(Number(evidence.checks.cbsemDialog.tolerance)) || Number(evidence.checks.cbsemDialog.tolerance) <= 0
    || !/Maximum likelihood; first loading fixed to 1/i.test(evidence.checks.cbsemDialog.estimator)
    || !/Single-group reflective raw-data CFA or recursive SEM/i.test(evidence.checks.cbsemDialog.scope)
    || !/listwise-standardized indicators/i.test(evidence.checks.cbsemDialog.scope)
    || evidence.checks.cbsemDialog.unsupportedControls !== 0 || blockers.length !== 0 || !evidence.checks.cbsemDialog.startEnabled) {
    throw new Error(`The focused CB-SEM dialog did not match the exact bounded raw/listwise/ML contract: ${JSON.stringify(evidence.checks.cbsemDialog)}`);
  }
  await capture(cbsemCaptureName(132, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, cbsemCaptureName(133, "running"), "CB-SEM / CFA")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 180_000);
  evidence.checks.cbsemProgress = await activeCapture;
  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed CB-SEM run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const fit = await readTable("Model fit");
  const standardized = await readTable("Standardized parameters");
  const unstandardized = await readTable("Unstandardized parameters");
  const residualCorrelations = await readTable("Residual correlations");
  const residualCovariances = await readTable("Residual covariances");
  const impliedCovariances = await readTable("Model-implied covariances");
  const modificationDiagnostics = await readTable("Residual-based modification diagnostics");
  const scope = await readTable("Calculation scope");
  const fitValues = Object.fromEntries(fit.values.map((row) => [row[0], row[1]]));
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [fit, standardized, unstandardized, residualCorrelations, residualCovariances, impliedCovariances, modificationDiagnostics, scope]
    .flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const resultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const modelEstimateItem = page.locator('.nd-result-tree [role="treeitem"]').filter({ hasText: /^Model estimates$/ });
  await modelEstimateItem.click();
  await page.getByRole("heading", { name: "Standardized model estimates", exact: true }).waitFor({ state: "visible", timeout: 10_000 });
  const diagram = {
    heading: compactVisibleText(await page.getByRole("heading", { name: "Standardized model estimates", exact: true }).textContent()),
    imageAlt: await page.locator(".nd-result-diagram-canvas img").getAttribute("alt"),
  };
  evidence.checks.cbsemResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    fit,
    standardized,
    unstandardized,
    residualCorrelations,
    residualCovariances,
    impliedCovariances,
    modificationDiagnostics,
    scope,
    fitValues,
    scopeValues,
    runDetails,
    resultProperties,
    diagram,
    noPlaceholder: !/\bN\/?A\b/i.test(resultsText),
    noGenericPlsTables: !/Path coefficients|Outer loadings|Construct reliability|Mediation|Prediction/i.test(treeItems.join(" ")),
  };
  const expectedTree = [
    "Graphical results", "Model estimates", "CB-SEM / CFA", "Model fit", "Standardized parameters", "Unstandardized parameters",
    "Residual correlations", "Residual covariances", "Model-implied covariances", "Residual-based modification diagnostics", "Calculation scope",
  ];
  if (initialSelectedTable !== "cbsem_fit" || JSON.stringify(treeItems) !== JSON.stringify(expectedTree)
    || fit.rows !== 13 || standardized.rows !== 23 || unstandardized.rows !== 23
    || residualCorrelations.rows !== 45 || residualCovariances.rows !== 45 || impliedCovariances.rows !== 45
    || modificationDiagnostics.rows !== 50 || scope.rows !== 14
    || JSON.stringify(fit.headers) !== JSON.stringify(["Fit measure", "Value"])
    || JSON.stringify(standardized.headers) !== JSON.stringify(["Parameter", "Type", "Std. LV", "Std. all"])
    || JSON.stringify(unstandardized.headers) !== JSON.stringify(["Parameter", "Type", "Estimate", "SE", "z", "p (two-sided)", "Status"])
    || !["CFI", "TLI", "RMSEA", "SRMR", "AIC", "BIC"].every((key) => Object.hasOwn(fitValues, key) && fitValues[key] !== "")
    || !standardized.values.some((row) => row[1] === "Structural path") || !standardized.values.some((row) => row[1] === "Loading")
    || !unstandardized.values.some((row) => row[6] === "Fixed for marker identification")
    || !unstandardized.values.some((row) => row[6] === "Estimated")
    || scopeValues["Model type"] !== "Recursive structural equation model" || scopeValues.Estimator !== "Maximum likelihood"
    || scopeValues.Input !== "Raw case-level data; indicators standardized after listwise filtering"
    || scopeValues.Identification !== "First loading fixed to 1 for each latent factor"
    || scopeValues["Mean structure"] !== "Not estimated" || scopeValues["Analyzed observations"] !== "240"
    || scopeValues.Converged !== "Yes" || scopeValues["Estimator method version"] !== cbsemMethodVersion
    || scopeValues["Fit method version"] !== cbsemFitMethodVersion
    || scopeValues["Modification-diagnostic version"] !== cbsemModificationMethodVersion
    || runDetails.properties.Method !== "CB-SEM / CFA" || runDetails.properties["Method version"] !== cbsemProvenanceMethodVersion
    || runDetails.properties["Model type"] !== "Recursive structural equation model"
    || runDetails.properties.Estimator !== "Maximum likelihood" || runDetails.properties["Complete cases"] !== "240"
    || runDetails.properties.Converged !== "Yes" || runDetails.properties.Input !== "Raw case-level data"
    || runDetails.properties["Missing data"] !== "Listwise deletion"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing")
    || resultProperties.Method !== "CB-SEM / CFA" || resultProperties["Model type"] !== "Recursive structural equation model"
    || resultProperties.Estimator !== "Maximum likelihood" || resultProperties["Complete cases"] !== "240"
    || diagram.heading !== "Standardized model estimates" || !/Standardized model estimates/i.test(diagram.imageAlt ?? "")
    || !evidence.checks.cbsemResult.noPlaceholder || !evidence.checks.cbsemResult.noGenericPlsTables) {
    throw new Error(`The completed CB-SEM result did not expose the exact ML fit, parameter, residual, diagnostic, scope, and standardized-diagram contract: ${JSON.stringify(evidence.checks.cbsemResult)}`);
  }
  await openResultTable("Standardized parameters");
  await capture(cbsemCaptureName(134, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const exportButtonTexts = await exportDialog.locator(".nd-export-list button").evaluateAll((buttons) => buttons.map((button) => (
    button.innerText.replace(/\s+/g, " ").trim()
  )));
  const expectedFormats = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"];
  const tableTitles = [
    "Model fit", "Standardized parameters", "Unstandardized parameters", "Residual correlations", "Residual covariances",
    "Model-implied covariances", "Residual-based modification diagnostics", "Calculation scope",
  ];
  const expectedSheets = [
    "Model fit", "Standardized parameters", "Unstandardized parameters", "Residual correlations", "Residual covariances",
    "Model-implied covariances", "Residual-based modification dia", "Calculation scope", "Run provenance",
  ];
  evidence.checks.cbsemExport = {
    formats: expectedFormats,
    buttonTexts: exportButtonTexts,
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    everyFormatPresentOnce: expectedFormats.every((label) => exportButtonTexts.filter((text) => text.startsWith(label)).length === 1),
    xlsxEnabled: await xlsxExport.isEnabled(),
    nativeXlsx: null,
  };
  if (evidence.checks.cbsemExport.buttonCount !== 6 || !evidence.checks.cbsemExport.everyFormatPresentOnce
    || !evidence.checks.cbsemExport.xlsxEnabled) {
    throw new Error(`The completed CB-SEM result did not expose exactly six model-and-table export formats: ${JSON.stringify(evidence.checks.cbsemExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [...tableTitles, "Run provenance", "Maximum likelihood", cbsemMethodVersion, cbsemFitMethodVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native CB-SEM XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native CB-SEM XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.cbsemExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.cbsemExport.nativeXlsx.appFeedback !== expectedFeedback
      || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)) {
      throw new Error(`The genuine CB-SEM XLSX did not contain every result and provenance sheet exactly once: ${JSON.stringify(evidence.checks.cbsemExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(cbsemCaptureName(135, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedCbsemArchive(cbsemProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(cbsemProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /CB-SEM \/ CFA/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened CB-SEM result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedFitRows = await openResultTable("Model fit");
  const reopenedStandardizedRows = await openResultTable("Standardized parameters");
  const reopenedModificationRows = await openResultTable("Residual-based modification diagnostics");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.cbsemSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    fitRows: reopenedFitRows,
    standardizedRows: reopenedStandardizedRows,
    modificationRows: reopenedModificationRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.cbsemSaveReopen.sameRunRestored || reopenedFitRows !== 13 || reopenedStandardizedRows !== 23
    || reopenedModificationRows !== 50 || reopenedScopeRows !== 14) {
    throw new Error(`The exact CB-SEM ML run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.cbsemSaveReopen)}`);
  }
  await openResultTable("Standardized parameters");
  await capture(cbsemCaptureName(136, "reopened"));
}

async function runFocusedGscaAcceptance() {
  if (!requestedGscaNativeExportPath) {
    throw new Error("QUICKPLS_GSCA_NATIVE_EXPORT_PATH is required for focused packaged GSCA acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedGscaNativeExportPath,
    "QUICKPLS_GSCA_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: gscaProjectName, path: gscaProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(gscaProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialGscaArchive(gscaProjectPath);
  evidence.checks.gscaFixture = {
    projectPath: gscaProjectPath,
    sourceCsv: gscaFixtureCsvPath,
    status,
    cases: status.includes("140 cases") ? 140 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  if (evidence.checks.gscaFixture.cases !== 140 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused GSCA fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.gscaFixture)}`);
  }
  await capture(gscaCaptureName(140, "fixture-data"));

  evidence.checks.gscaInitialModelCreation = await createInitialEditableModel(gscaProjectName, gscaModelName);
  await buildTwoConstructGscaModel();
  const nodes = page.locator(".react-flow__node-latent");
  const formativeChecked = await nodes.nth(0).locator(".smartpls-latent-node.formative").count() === 1;
  const reflectiveChecked = await nodes.nth(1).locator(".smartpls-latent-node.reflective").count() === 1;
  evidence.checks.gscaModel = {
    constructs: await nodes.count(),
    constructLabels: (await nodes.allTextContents()).map(compactVisibleText),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    formativeChecked,
    reflectiveChecked,
  };
  if (evidence.checks.gscaModel.constructs !== 2 || evidence.checks.gscaModel.assignedIndicators !== 5
    || evidence.checks.gscaModel.structuralPaths !== 1 || !formativeChecked || !reflectiveChecked
    || !evidence.checks.gscaModel.constructLabels.some((label) => label.includes("G formative component"))
    || !evidence.checks.gscaModel.constructLabels.some((label) => label.includes("H reflective component"))) {
    throw new Error(`The visible GSCA authoring workflow did not create the exact formative G -> reflective H model: ${JSON.stringify(evidence.checks.gscaModel)}`);
  }
  await capture(gscaCaptureName(141, "model"));
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const calculation = await openCalculationFromToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-gsca").click();
  const start = calculation.getByRole("button", { name: "Start GSCA", exact: true });
  const blockers = await calculation.locator(".nd-blocker li").allTextContents();
  evidence.checks.gscaDialog = {
    catalogCount: await options.count(),
    optionLabels: (await options.locator("strong").allTextContents()).map(compactVisibleText),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-component_models").textContent()),
    weighting: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Weighting scheme" }).locator("strong").textContent()),
    resultData: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Result data" }).locator("strong").textContent()),
    estimator: compactVisibleText(await calculation.locator("#nd-calculation-gsca-estimator").textContent()),
    scope: compactVisibleText(await calculation.locator("#nd-calculation-gsca-scope").textContent()),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-max-iterations", "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations", "#nd-calculation-nca-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight", "#nd-calculation-group-column",
    ].join(", ")).count(),
    blockers,
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.gscaDialog.catalogCount !== 14
    || JSON.stringify(evidence.checks.gscaDialog.optionLabels) !== JSON.stringify(expectedOptionLabels)
    || evidence.checks.gscaDialog.selectedMethod !== "GSCA" || evidence.checks.gscaDialog.category !== "Component models"
    || evidence.checks.gscaDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.gscaDialog.resultData !== "Standardized (fixed)"
    || !/Joint global least-squares alternating least squares; fixed \+1 initialization/i.test(evidence.checks.gscaDialog.estimator)
    || !/3,000 maximum iterations/i.test(evidence.checks.gscaDialog.scope)
    || !/1e-7 objective-and-weight stop criterion/i.test(evidence.checks.gscaDialog.scope)
    || !/No controls, covariance paths, interactions, higher-order constructs, case weights, multigroup analysis, or inference/i.test(evidence.checks.gscaDialog.scope)
    || evidence.checks.gscaDialog.unsupportedControls !== 0 || blockers.length !== 0 || !evidence.checks.gscaDialog.startEnabled) {
    throw new Error(`The focused GSCA dialog did not match the exact bounded ALS v2 contract: ${JSON.stringify(evidence.checks.gscaDialog)}`);
  }
  await capture(gscaCaptureName(142, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, gscaCaptureName(143, "running"), "GSCA")
    .then((state) => ({ captured: true, ...state }));
  await start.click();
  await waitForSurface("results", 180_000);
  evidence.checks.gscaProgress = await activeCapture;
  if (!evidence.checks.gscaProgress.captured || !evidence.checks.gscaProgress.status
    || !evidence.checks.gscaProgress.phase || !evidence.checks.gscaProgress.message) {
    throw new Error(`GSCA did not expose a genuine queued, validating, or running lifecycle state: ${JSON.stringify(evidence.checks.gscaProgress)}`);
  }

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed GSCA run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const fit = await readTable("Model fit and convergence");
  const paths = await readTable("Structural path coefficients");
  const rSquared = await readTable("Endogenous construct R²");
  const loadings = await readTable("Measurement loadings");
  const weights = await readTable("Component weights");
  const scope = await readTable("Calculation scope");
  const fitValues = Object.fromEntries(fit.values.map((row) => [row[0], row[1]]));
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [fit, paths, rSquared, loadings, weights, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const resultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await page.getByRole("treeitem", { name: "Model estimates", exact: true }).click();
  await page.getByRole("heading", { name: "Model estimates", exact: true }).waitFor({ state: "visible", timeout: 10_000 });
  const diagram = {
    heading: compactVisibleText(await page.getByRole("heading", { name: "Model estimates", exact: true }).textContent()),
    imageAlt: await page.locator(".nd-result-diagram-canvas img").getAttribute("alt"),
  };
  evidence.checks.gscaResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    fit,
    paths,
    rSquared,
    loadings,
    weights,
    scope,
    fitValues,
    scopeValues,
    runDetails,
    resultProperties,
    diagram,
    noPlaceholder: !/\bN\/?A\b/i.test(resultsText),
    noGenericPlsOrInference: !/Final results|Quality criteria|Assessment|Inference|Bootstrap|Permutation/i.test(treeItems.join(" ")),
  };
  const expectedTree = [
    "Graphical results", "Model estimates", "GSCA component model", "Model fit and convergence", "Structural path coefficients",
    "Endogenous construct R²", "Measurement loadings", "Component weights", "Calculation scope",
  ];
  if (initialSelectedTable !== "gsca_fit" || JSON.stringify(treeItems) !== JSON.stringify(expectedTree)
    || fit.rows !== 12 || paths.rows !== 1 || rSquared.rows !== 1 || loadings.rows !== 5 || weights.rows !== 5 || scope.rows !== 11
    || JSON.stringify(fit.headers) !== JSON.stringify(["Measure", "Value"])
    || JSON.stringify(paths.headers) !== JSON.stringify(["Path", "Coefficient"])
    || JSON.stringify(loadings.headers) !== JSON.stringify(["Construct", "Indicator", "Measurement model", "Loading"])
    || JSON.stringify(weights.headers) !== JSON.stringify(["Construct", "Indicator", "Measurement model", "Weight"])
    || fitValues.Converged !== "Yes" || fitValues["ALS iterations"] !== "4"
    || !["Global FIT", "Adjusted FIT", "Measurement FIT", "Structural FIT", "GFI", "SRMR", "Objective", "Final objective-and-weight change"].every((key) => Object.hasOwn(fitValues, key) && Number.isFinite(Number(fitValues[key])))
    || paths.values[0]?.[0] !== "H reflective component ← G formative component"
    || !loadings.values.every((row) => row[2] === (row[0] === "G formative component" ? "Formative" : "Reflective"))
    || !weights.values.every((row) => row[2] === (row[0] === "G formative component" ? "Formative" : "Reflective"))
    || scopeValues.Estimator !== "Joint global least-squares alternating least squares"
    || scopeValues["Method version"] !== gscaMethodVersion || scopeValues["Algorithm version"] !== gscaAlgorithmVersion
    || scopeValues.Inference !== "Point estimates only; no bootstrap or permutation inference"
    || runDetails.properties.Method !== "GSCA" || runDetails.properties["Method version"] !== gscaMethodVersion
    || runDetails.properties.Estimator !== "Joint global least-squares ALS" || runDetails.properties["Complete cases"] !== "140"
    || runDetails.properties.Converged !== "Yes" || runDetails.properties["ALS iterations"] !== "4"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing") || Object.hasOwn(runDetails.properties, "Recorded seed")
    || resultProperties.Method !== "GSCA" || resultProperties.Estimator !== "Joint global least-squares ALS"
    || resultProperties["Complete cases"] !== "140" || resultProperties.Converged !== "Yes" || resultProperties["ALS iterations"] !== "4"
    || diagram.heading !== "Model estimates" || !/Model estimates for GSCA run/i.test(diagram.imageAlt ?? "")
    || !evidence.checks.gscaResult.noPlaceholder || !evidence.checks.gscaResult.noGenericPlsOrInference) {
    throw new Error(`The completed GSCA result did not expose the exact ALS fit, path, measurement, scope, and diagram contract: ${JSON.stringify(evidence.checks.gscaResult)}`);
  }
  await openResultTable("Model fit and convergence");
  await capture(gscaCaptureName(144, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const exportButtonTexts = await exportDialog.locator(".nd-export-list button").evaluateAll((buttons) => buttons.map((button) => button.innerText.replace(/\s+/g, " ").trim()));
  const expectedFormats = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"];
  const tableTitles = ["Model fit and convergence", "Structural path coefficients", "Endogenous construct R²", "Measurement loadings", "Component weights", "Calculation scope"];
  const expectedSheets = [...tableTitles, "Run provenance"];
  evidence.checks.gscaExport = {
    formats: expectedFormats,
    buttonTexts: exportButtonTexts,
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    everyFormatPresentOnce: expectedFormats.every((label) => exportButtonTexts.filter((text) => text.startsWith(label)).length === 1),
    xlsxEnabled: await xlsxExport.isEnabled(),
    nativeXlsx: null,
  };
  if (evidence.checks.gscaExport.buttonCount !== 6 || !evidence.checks.gscaExport.everyFormatPresentOnce || !evidence.checks.gscaExport.xlsxEnabled) {
    throw new Error(`The completed GSCA result did not expose exactly six model-and-table export formats: ${JSON.stringify(evidence.checks.gscaExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [...tableTitles, "Run provenance", "Joint global least-squares alternating least squares", gscaMethodVersion, gscaAlgorithmVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native GSCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native GSCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.gscaExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.gscaExport.nativeXlsx.appFeedback !== expectedFeedback
      || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)) {
      throw new Error(`The genuine GSCA XLSX did not contain every result and provenance sheet exactly once: ${JSON.stringify(evidence.checks.gscaExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(gscaCaptureName(145, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedGscaArchive(gscaProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(gscaProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /^GSCA run$/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened GSCA result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedFitRows = await openResultTable("Model fit and convergence");
  const reopenedLoadingRows = await openResultTable("Measurement loadings");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.gscaSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    fitRows: reopenedFitRows,
    loadingRows: reopenedLoadingRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.gscaSaveReopen.sameRunRestored || reopenedFitRows !== 12 || reopenedLoadingRows !== 5 || reopenedScopeRows !== 11) {
    throw new Error(`The exact GSCA ALS v2 run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.gscaSaveReopen)}`);
  }
  await openResultTable("Model fit and convergence");
  await capture(gscaCaptureName(146, "reopened"));
}

async function runFocusedOlsAcceptance() {
  if (!requestedOlsNativeExportPath) {
    throw new Error("QUICKPLS_OLS_NATIVE_EXPORT_PATH is required for focused packaged OLS acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedOlsNativeExportPath,
    "QUICKPLS_OLS_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: olsProjectName, path: olsProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(olsProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialOlsArchive(olsProjectPath);
  evidence.checks.olsFixture = {
    projectPath: olsProjectPath,
    status,
    cases: status.includes("140 cases") ? 140 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  if (evidence.checks.olsFixture.cases !== 140 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused OLS fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.olsFixture)}`);
  }
  await capture(olsCaptureName(120, "fixture-data"));

  const calculation = await openAnalysisFromDataToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-regression").click();
  const olsSettings = calculation.locator(".nd-ols-settings");
  await olsSettings.waitFor({ state: "visible", timeout: 10_000 });
  const outcome = calculation.locator("#nd-calculation-ols-outcome");
  await outcome.selectOption(olsOutcome);
  const roleFieldsets = olsSettings.locator("fieldset.nd-pca-variables");
  const roleLabels = (fieldset) => fieldset.locator("label");
  for (const fieldset of [roleFieldsets.nth(0), roleFieldsets.nth(1)]) {
    const checked = fieldset.locator('input[type="checkbox"]:checked');
    while (await checked.count()) await checked.first().uncheck();
  }
  for (const variable of olsPredictors) {
    const label = roleLabels(roleFieldsets.nth(0)).filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`OLS predictor ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  for (const variable of olsControls) {
    const label = roleLabels(roleFieldsets.nth(1)).filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`OLS control ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  const inspectRole = async (fieldset) => fieldset.locator("label").evaluateAll((labels) => labels.filter((label) => (
    label.querySelector('input[type="checkbox"]')?.checked
  )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
  const selectedPredictors = await inspectRole(roleFieldsets.nth(0));
  const selectedControls = await inspectRole(roleFieldsets.nth(1));
  const noteValue = async (label) => compactVisibleText(await calculation.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const start = calculation.getByRole("button", { name: "Start OLS regression", exact: true });
  const blockerText = compactVisibleText(await calculation.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
  evidence.checks.olsDialog = {
    catalogCount: await options.count(),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-standalone").textContent()),
    outcome: await outcome.inputValue(),
    selectedPredictors,
    selectedControls,
    calculationBasis: await noteValue("Calculation basis"),
    variableData: await noteValue("Variable data"),
    uncertainty: await noteValue("Uncertainty"),
    validatedScope: await noteValue("Validated scope"),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance", "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight",
    ].join(", ")).count(),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    blockerText,
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(blockerText),
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.olsDialog.catalogCount !== 12
    || evidence.checks.olsDialog.selectedMethod !== "Ordinary Least Squares Regression"
    || evidence.checks.olsDialog.category !== "Standalone analysis"
    || evidence.checks.olsDialog.outcome !== olsOutcome
    || JSON.stringify(selectedPredictors) !== JSON.stringify(olsPredictors)
    || JSON.stringify(selectedControls) !== JSON.stringify(olsControls)
    || evidence.checks.olsDialog.calculationBasis !== "Raw-value OLS with intercept (fixed)"
    || evidence.checks.olsDialog.variableData !== "Unstandardized numeric values (fixed)"
    || evidence.checks.olsDialog.uncertainty !== "HC3 robust SE; two-sided 95% CI (fixed)"
    || !/Raw numeric ordinary least squares with an intercept/i.test(evidence.checks.olsDialog.validatedScope)
    || evidence.checks.olsDialog.unsupportedControls !== 0 || evidence.checks.olsDialog.blockers.length !== 0
    || !evidence.checks.olsDialog.noModelBlocker || !evidence.checks.olsDialog.startEnabled) {
    throw new Error(`The focused OLS dialog did not match the exact raw/listwise/HC3 model-free contract: ${JSON.stringify(evidence.checks.olsDialog)}`);
  }
  await capture(olsCaptureName(121, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, olsCaptureName(122, "running"), "standalone OLS")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 120_000);
  evidence.checks.olsProgress = await activeCapture;
  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed OLS run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const coefficients = await readTable("Coefficients");
  const fit = await readTable("Model fit");
  const scope = await readTable("Calculation scope");
  const coefficientValuesValid = coefficients.values.every((row) => row.slice(1).every((value, index) => (
    index === 3
      ? (/^<0\.0001$/.test(value) || (Number.isFinite(Number(value)) && Number(value) >= 0 && Number(value) <= 1))
      : Number.isFinite(Number(value))
  )));
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [coefficients, fit, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const editDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/i });
  const editModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/i });
  evidence.checks.olsResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    coefficients,
    fit,
    scope,
    scopeValues,
    runDetails,
    editDataCommand: { count: await editDataCommand.count(), enabled: await editDataCommand.isEnabled().catch(() => false) },
    editModelCommand: { count: await editModelCommand.count() },
    noPlaceholder: !/\bN\/?A\b/i.test(resultsText),
    noSemResultGroups: !/Model estimates|Quality criteria|Mediation|Moderation|Prediction/i.test(treeItems.join(" ")),
  };
  if (initialSelectedTable !== "ols_coefficients"
    || JSON.stringify(treeItems) !== JSON.stringify(["OLS regression", "Coefficients", "Model fit", "Calculation scope"])
    || coefficients.rows !== 4
    || JSON.stringify(coefficients.headers) !== JSON.stringify(["Term", "Estimate", "HC3 SE", "t", "p (two-sided)", "95% CI lower", "95% CI upper"])
    || JSON.stringify(coefficients.values.map((row) => row[0])) !== JSON.stringify(["Intercept", "x", "m", "z"])
    || !coefficientValuesValid
    || fit.rows !== 1 || JSON.stringify(fit.headers) !== JSON.stringify(["Observations", "R\u00B2", "Adjusted R\u00B2", "F", "RMSE", "AIC", "BIC"])
    || fit.values[0]?.[0] !== "140" || fit.values[0]?.slice(1).some((value) => !Number.isFinite(Number(value)))
    || scope.rows !== 11 || scopeValues.Outcome !== olsOutcome || scopeValues.Predictors !== olsPredictors.join(", ")
    || scopeValues.Controls !== olsControls.join(", ") || scopeValues["Analyzed observations"] !== "140"
    || scopeValues["Standard errors"] !== "HC3 heteroskedasticity-consistent"
    || scopeValues["Confidence intervals"] !== "Two-sided 95%" || scopeValues["Method version"] !== olsMethodVersion
    || runDetails.properties.Method !== "Ordinary Least Squares Regression"
    || runDetails.properties["Method version"] !== olsMethodVersion || runDetails.properties.Outcome !== olsOutcome
    || runDetails.properties.Predictors !== olsPredictors.join(", ") || runDetails.properties.Controls !== olsControls.join(", ")
    || runDetails.properties.Observations !== "140" || runDetails.properties["Standard errors"] !== "HC3 robust"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing")
    || evidence.checks.olsResult.editDataCommand.count !== 1 || !evidence.checks.olsResult.editDataCommand.enabled
    || evidence.checks.olsResult.editModelCommand.count !== 0
    || !evidence.checks.olsResult.noPlaceholder || !evidence.checks.olsResult.noSemResultGroups) {
    throw new Error(`The completed OLS result did not expose the exact HC3 coefficient, fit, scope, and model-free return boundary: ${JSON.stringify(evidence.checks.olsResult)}`);
  }
  await openResultTable("Coefficients");
  await capture(olsCaptureName(123, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const tableTitles = ["Coefficients", "Model fit", "Calculation scope", "Fitted values and residuals"];
  const expectedSheets = [...tableTitles, "Run provenance"];
  evidence.checks.olsExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    modelDiagramFormats: await exportDialog.getByRole("button", { name: /diagram|svg/i }).count(),
    nativeXlsx: null,
  };
  if (!evidence.checks.olsExport.xlsxEnabled || evidence.checks.olsExport.buttonCount !== 5
    || evidence.checks.olsExport.modelDiagramFormats !== 0) {
    throw new Error(`The model-free OLS result did not expose exactly five table-only export formats: ${JSON.stringify(evidence.checks.olsExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [...tableTitles, "Run provenance", "HC3 SE", olsMethodVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native OLS XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native OLS XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.olsExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.olsExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine OLS XLSX did not contain every result, fitted/residual, and provenance sheet: ${JSON.stringify(evidence.checks.olsExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(olsCaptureName(124, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedOlsArchive(olsProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(olsProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Ordinary Least Squares Regression/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened OLS result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedCoefficientRows = await openResultTable("Coefficients");
  const reopenedFitRows = await openResultTable("Model fit");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.olsSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    coefficientRows: reopenedCoefficientRows,
    fitRows: reopenedFitRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.olsSaveReopen.sameRunRestored || reopenedCoefficientRows !== 4
    || reopenedFitRows !== 1 || reopenedScopeRows !== 11) {
    throw new Error(`The exact model-free OLS run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.olsSaveReopen)}`);
  }
  await openResultTable("Coefficients");
  await capture(olsCaptureName(125, "reopened"));
}

async function runFocusedPcaAcceptance() {
  if (!requestedPcaNativeExportPath) {
    throw new Error("QUICKPLS_PCA_NATIVE_EXPORT_PATH is required for focused packaged PCA acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const pcaExportTargetPath = await validateRequestedNativeExportPath(
    requestedPcaNativeExportPath,
    "QUICKPLS_PCA_NATIVE_EXPORT_PATH",
  );

  await seedRecentProject({
    name: pcaProjectName,
    path: pcaProjectPath,
    openedAt: "2026-08-12T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(pcaProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialPcaArchive(pcaProjectPath);
  evidence.checks.pcaFixture = {
    projectPath: pcaProjectPath,
    status,
    cases: status.includes("140 cases") ? 140 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  if (evidence.checks.pcaFixture.cases !== 140 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused PCA fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.pcaFixture)}`);
  }
  await capture(pcaCaptureName(110, "fixture-data"));

  const calculation = await openAnalysisFromDataToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  const pcaOption = calculation.locator("#nd-calculation-method-pca");
  await pcaOption.click();
  await calculation.locator(".nd-pca-settings").waitFor({ state: "visible", timeout: 10_000 });

  const variableLabels = calculation.locator(".nd-pca-variable-list label");
  const availableVariables = await variableLabels.evaluateAll((labels) => labels.map((label) => (
    label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""
  )));
  await calculation.getByRole("button", { name: "Clear", exact: true }).click();
  for (const variable of pcaVariables) {
    const label = variableLabels.filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`PCA variable ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  const selectedVariables = await variableLabels.evaluateAll((labels) => labels.filter((label) => (
    label.querySelector('input[type="checkbox"]')?.checked
  )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
  const retention = calculation.locator("#nd-calculation-pca-rule");
  await retention.selectOption("variance_threshold");
  const threshold = calculation.locator("#nd-calculation-pca-threshold");
  await threshold.fill(String(pcaVarianceThreshold * 100));
  const noteValue = async (label) => compactVisibleText(await calculation.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const start = calculation.getByRole("button", { name: "Start principal component analysis", exact: true });
  const blockerText = compactVisibleText(await calculation.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
  evidence.checks.pcaDialog = {
    catalogCount: await options.count(),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-standalone").textContent()),
    availableVariables,
    selectedVariables,
    componentRule: await retention.inputValue(),
    varianceThresholdPercent: await threshold.inputValue(),
    calculationBasis: await noteValue("Calculation basis"),
    variableData: await noteValue("Variable data"),
    validatedScope: await noteValue("Validated scope"),
    maxIterationsCount: await calculation.locator("#nd-calculation-max-iterations").count(),
    toleranceCount: await calculation.locator("#nd-calculation-tolerance").count(),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-bootstrap-samples",
      "#nd-calculation-permutations", "#nd-calculation-nca-permutations", "#nd-calculation-seed",
      "#nd-calculation-workers", "#nd-calculation-case-weight", "#nd-calculation-group-column",
    ].join(", ")).count(),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    blockerText,
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(blockerText),
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.pcaDialog.catalogCount !== 12
    || evidence.checks.pcaDialog.selectedMethod !== "Principal Component Analysis"
    || evidence.checks.pcaDialog.category !== "Standalone analysis"
    || JSON.stringify(availableVariables) !== JSON.stringify(expectedColumns.slice(1))
    || JSON.stringify(selectedVariables) !== JSON.stringify(pcaVariables)
    || evidence.checks.pcaDialog.componentRule !== "variance_threshold"
    || evidence.checks.pcaDialog.varianceThresholdPercent !== "95"
    || evidence.checks.pcaDialog.calculationBasis !== "Correlation matrix (fixed)"
    || evidence.checks.pcaDialog.variableData !== "Standardized numeric values (fixed)"
    || !/Correlation-matrix PCA of 2 to 50 selected numeric variables/i.test(evidence.checks.pcaDialog.validatedScope)
    || evidence.checks.pcaDialog.maxIterationsCount !== 0 || evidence.checks.pcaDialog.toleranceCount !== 0
    || evidence.checks.pcaDialog.unsupportedControls !== 0 || evidence.checks.pcaDialog.blockers.length !== 0
    || !evidence.checks.pcaDialog.noModelBlocker || !evidence.checks.pcaDialog.startEnabled) {
    throw new Error(`The focused PCA dialog did not match the exact model-free variance-threshold contract: ${JSON.stringify(evidence.checks.pcaDialog)}`);
  }
  await capture(pcaCaptureName(111, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, pcaCaptureName(112, "running"), "standalone PCA")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 120_000);
  evidence.checks.pcaProgress = await activeCapture;

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const pcaRunId = await page.locator(".nd-run-select select").inputValue();
  if (!pcaRunId) throw new Error("The completed PCA run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"),
        (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const summary = await readTable("Component summary");
  const loadings = await readTable("Component loadings and weights");
  const scope = await readTable("Calculation scope");
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [summary, loadings, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const editDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/i });
  const editModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/i });
  evidence.checks.pcaResult = {
    runId: pcaRunId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    summary,
    loadings,
    scope,
    scopeValues,
    runDetails,
    editDataCommand: { count: await editDataCommand.count(), enabled: await editDataCommand.isEnabled().catch(() => false) },
    editModelCommand: { count: await editModelCommand.count() },
    noPlaceholder: !/\bN\/A\b/i.test(resultsText),
    noSemResultGroups: !/Model estimates|Quality criteria|Mediation|Moderation|Prediction/i.test(treeItems.join(" ")),
  };
  const cumulativePercentages = summary.values.map((row) => Number(row[3]?.replace("%", "")));
  const loadingIdentities = new Set(loadings.values.map((row) => `${row[0]}\u0000${row[1]}`));
  if (initialSelectedTable !== "pca_component_summary"
    || JSON.stringify(treeItems) !== JSON.stringify(["Principal components", "Component summary", "Component loadings and weights", "Calculation scope"])
    || summary.rows !== 4
    || JSON.stringify(summary.headers) !== JSON.stringify(["Component", "Eigenvalue", "Explained variance", "Cumulative variance"])
    || JSON.stringify(summary.values.map((row) => row[0])) !== JSON.stringify(["PC1", "PC2", "PC3", "PC4"])
    || !cumulativePercentages.every(Number.isFinite) || cumulativePercentages[2] >= 95 || cumulativePercentages[3] < 95
    || loadings.rows !== 20 || loadingIdentities.size !== 20
    || JSON.stringify([...new Set(loadings.values.map((row) => row[0]))]) !== JSON.stringify(pcaVariables)
    || scope.rows !== 10 || scopeValues.Variables !== "5" || scopeValues["Analyzed observations"] !== "140"
    || scopeValues["Retention rule"] !== "Cumulative variance threshold" || scopeValues["Retained components"] !== "4"
    || scopeValues["Stored component scores"] !== "560" || scopeValues.Rotation !== "None" || scopeValues["Method version"] !== pcaMethodVersion
    || runDetails.properties.Method !== "Principal Component Analysis" || runDetails.properties["Method version"] !== pcaMethodVersion
    || runDetails.properties.Variables !== "5" || runDetails.properties["Retention rule"] !== "Cumulative variance threshold"
    || runDetails.properties["Retained components"] !== "4" || runDetails.properties.Observations !== "140"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing")
    || evidence.checks.pcaResult.editDataCommand.count !== 1 || !evidence.checks.pcaResult.editDataCommand.enabled
    || evidence.checks.pcaResult.editModelCommand.count !== 0
    || !evidence.checks.pcaResult.noPlaceholder || !evidence.checks.pcaResult.noSemResultGroups) {
    throw new Error(`The completed PCA result did not expose the exact four-component crossing result, tables, and model-free return boundary: ${JSON.stringify(evidence.checks.pcaResult)}`);
  }
  await openResultTable("Component summary");
  await capture(pcaCaptureName(113, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const pcaTableTitles = ["Component summary", "Component loadings and weights", "Calculation scope", "Component scores"];
  const expectedPcaSheets = [...pcaTableTitles, "Run provenance"];
  const expectedExportFormats = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Print / PDF"];
  const exportButtonTexts = await exportDialog.locator(".nd-export-list button").evaluateAll((buttons) => buttons.map((button) => (
    button.innerText.replace(/\s+/g, " ").trim()
  )));
  evidence.checks.pcaExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    formats: expectedExportFormats,
    buttonTexts: exportButtonTexts,
    everyFormatPresentOnceWhenReadable: exportButtonTexts.length === 0 ? null : expectedExportFormats.every((label) => (
      exportButtonTexts.filter((text) => text.startsWith(label)).length === 1
    )),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    modelDiagramFormats: await exportDialog.getByRole("button", { name: /diagram|svg/i }).count(),
    nativeXlsx: null,
  };
  if (!evidence.checks.pcaExport.xlsxEnabled || evidence.checks.pcaExport.buttonCount !== 5
    || evidence.checks.pcaExport.everyFormatPresentOnceWhenReadable === false
    || evidence.checks.pcaExport.modelDiagramFormats !== 0) {
    throw new Error(`The model-free PCA result did not expose exactly five table-only export formats: ${JSON.stringify(evidence.checks.pcaExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: pcaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: expectedPcaSheets,
    expectedSharedStrings: [...pcaTableTitles, "Run provenance", "Cumulative variance threshold", pcaMethodVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native PCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native PCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(pcaExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(pcaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(pcaExportTargetPath);
    evidence.checks.pcaExport.nativeXlsx = {
      attempted: true,
      targetPath: pcaExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.pcaExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedPcaSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine PCA XLSX did not contain every result, full score, and provenance sheet: ${JSON.stringify(evidence.checks.pcaExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(pcaCaptureName(114, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedPcaArchive(pcaProjectPath, pcaRunId);
  await reloadToLauncher();
  await openRecentProject(pcaProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Principal Component Analysis/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened PCA result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedSummaryRows = await openResultTable("Component summary");
  const reopenedLoadingRows = await openResultTable("Component loadings and weights");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.pcaSaveReopen = {
    expectedRunId: pcaRunId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === pcaRunId,
    summaryRows: reopenedSummaryRows,
    loadingRows: reopenedLoadingRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.pcaSaveReopen.sameRunRestored || reopenedSummaryRows !== 4
    || reopenedLoadingRows !== 20 || reopenedScopeRows !== 10) {
    throw new Error(`The exact model-free PCA run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.pcaSaveReopen)}`);
  }
  await openResultTable("Component summary");
  await capture(pcaCaptureName(115, "reopened"));
}

async function runFocusedHigherOrderAcceptance() {
  if (!requestedHocNativeExportPath) {
    throw new Error("QUICKPLS_HOC_NATIVE_EXPORT_PATH is required for focused packaged HOC acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const hocExportTargetPath = await validateRequestedNativeExportPath(
    requestedHocNativeExportPath,
    "QUICKPLS_HOC_NATIVE_EXPORT_PATH",
  );

  await seedRecentProject({
    name: hocProjectName,
    path: hocProjectPath,
    openedAt: "2026-08-12T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(hocProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.hocFixture = {
    projectPath: hocProjectPath,
    status,
    cases: status.includes("120 cases") ? 120 : null,
    columns,
  };
  if (evidence.checks.hocFixture.cases !== 120 || JSON.stringify(columns) !== JSON.stringify(["#", "x1", "z1", "y1"])) {
    throw new Error(`The focused HOC fixture did not expose the expected 120-row x1/z1/y1 data: ${JSON.stringify(evidence.checks.hocFixture)}`);
  }
  await capture(hocCaptureName(100, "fixture-data"));

  evidence.checks.hocInitialModel = await createInitialEditableModel(hocProjectName, hocModelName);
  const authored = await buildThreeConstructHigherOrderModel();
  const hocCommand = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Higher-Order Construct/i });
  if (await hocCommand.count() !== 1 || !await hocCommand.isEnabled()) {
    throw new Error("The three eligible measurement-only constructs did not enable exactly one Higher-Order Construct toolbar command.");
  }
  await hocCommand.click();
  const hocDialog = page.locator('.nd-dialog-higher-order[role="dialog"]');
  await hocDialog.waitFor({ state: "visible", timeout: 10_000 });
  await hocDialog.getByLabel("Name", { exact: true }).fill("Organizational Capability");
  await hocDialog.getByLabel("Short name", { exact: true }).fill("OC");
  const capabilityCheckbox = hocDialog.getByRole("checkbox", { name: /Capability/ });
  const resourcesCheckbox = hocDialog.getByRole("checkbox", { name: /Resources/ });
  const performanceCheckbox = hocDialog.getByRole("checkbox", { name: /Performance/ });
  await capabilityCheckbox.check();
  await resourcesCheckbox.check();
  const createHoc = hocDialog.getByRole("button", { name: "Create higher-order construct", exact: true });
  const hocDialogText = compactVisibleText(await hocDialog.textContent());
  evidence.checks.hocDialog = {
    componentCount: await hocDialog.locator('input[type="checkbox"]').count(),
    capabilitySelected: await capabilityCheckbox.isChecked(),
    resourcesSelected: await resourcesCheckbox.isChecked(),
    performanceSelected: await performanceCheckbox.isChecked(),
    method: compactVisibleText(await hocDialog.locator(".nd-hoc-summary").textContent()),
    scope: compactVisibleText(await hocDialog.locator(".nd-dialog-note").textContent()),
    inferenceControls: await hocDialog.locator('[id*="bootstrap"], [id*="permutation"], [id*="confidence"], input[type="number"]').count(),
    createEnabled: await createHoc.isEnabled(),
    noBroaderClaim: !/repeated indicators|hybrid|bootstrapping available|permutation available/i.test(hocDialogText),
  };
  if (evidence.checks.hocDialog.componentCount !== 3
    || !evidence.checks.hocDialog.capabilitySelected || !evidence.checks.hocDialog.resourcesSelected || evidence.checks.hocDialog.performanceSelected
    || !/Reflective.+reflective disjoint two-stage/i.test(evidence.checks.hocDialog.method)
    || !/Stage 1.*component scores.*Stage 2.*generated HOC indicators/i.test(evidence.checks.hocDialog.method)
    || !/one HOC-to-outcome relationship/i.test(evidence.checks.hocDialog.scope)
    || !/no other structural path/i.test(evidence.checks.hocDialog.scope)
    || !/HOC bootstrapping and permutation inference remain unavailable/i.test(evidence.checks.hocDialog.scope)
    || evidence.checks.hocDialog.inferenceControls !== 0 || !evidence.checks.hocDialog.createEnabled || !evidence.checks.hocDialog.noBroaderClaim) {
    throw new Error(`The focused HOC dialog did not match the bounded disjoint two-stage point-estimate contract: ${JSON.stringify(evidence.checks.hocDialog)}`);
  }
  await capture(hocCaptureName(101, "dialog"));
  await createHoc.click();
  await hocDialog.waitFor({ state: "hidden", timeout: 10_000 });

  const allNodes = page.locator(".react-flow__node-latent");
  if (await allNodes.count() !== 4) throw new Error("HOC creation did not add exactly one generated higher-order construct.");
  const hocNode = allNodes.filter({ hasText: "Organizational Capability" });
  if (await hocNode.count() !== 1) throw new Error("The generated HOC was not visible under its authored name.");
  const hocId = await hocNode.getAttribute("data-id");
  if (!hocId) throw new Error("The generated HOC did not expose an immutable identifier.");
  const hocProperties = await page.locator('aside[aria-label="Model properties"] .nd-property-list').evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await createStructuralPath(allNodes, 3, 2, 1);
  const generatedPath = await structuralPaths().first().getAttribute("data-id");
  evidence.checks.hocModel = {
    constructs: await allNodes.count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    paths: await structuralPaths().count(),
    hocId,
    componentIds: [authored.ids.capability, authored.ids.resources],
    outcomeId: authored.ids.performance,
    generatedPath,
    properties: hocProperties,
  };
  if (evidence.checks.hocModel.constructs !== 4 || evidence.checks.hocModel.assignedIndicators !== 3 || evidence.checks.hocModel.paths !== 1
    || hocProperties.Type !== "Reflective–reflective HOC" || hocProperties.Method !== "Disjoint two-stage"
    || !hocProperties.Components?.includes("Capability") || !hocProperties.Components?.includes("Resources")
    || hocProperties.Indicators !== "Generated component scores") {
    throw new Error(`The visible HOC model did not retain its exact indicator-free semantics and single outgoing relationship: ${JSON.stringify(evidence.checks.hocModel)}`);
  }
  await capture(hocCaptureName(102, "model"));

  const calculation = await openCalculationFromToolbar();
  await calculation.getByRole("listbox", { name: "Available calculation methods", exact: true })
    .getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const start = calculation.getByRole("button", { name: "Start calculation", exact: true });
  evidence.checks.hocCalculation = {
    selectedMethod: compactVisibleText(await calculation.getByRole("option", { selected: true }).locator("strong").textContent()),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    startEnabled: await start.isEnabled(),
    bootstrapControls: await calculation.locator("#nd-calculation-bootstrap-samples").count(),
    permutationControls: await calculation.locator("#nd-calculation-permutations").count(),
  };
  if (evidence.checks.hocCalculation.selectedMethod !== "PLS-SEM Algorithm"
    || evidence.checks.hocCalculation.blockers.length !== 0 || !evidence.checks.hocCalculation.startEnabled
    || evidence.checks.hocCalculation.bootstrapControls !== 0 || evidence.checks.hocCalculation.permutationControls !== 0) {
    throw new Error(`The bounded HOC model was not runnable only through the ordinary PLS-SEM Algorithm workflow: ${JSON.stringify(evidence.checks.hocCalculation)}`);
  }
  const activeCapture = captureActiveCalculation(calculation, hocCaptureName(103, "running"), "two-stage HOC")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 120_000);
  evidence.checks.hocProgress = await activeCapture;

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const hocRunId = await page.locator(".nd-run-select select").inputValue();
  if (!hocRunId) throw new Error("The completed HOC run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const componentRows = await openResultTable("Higher-order component relationships");
  const componentHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const componentValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const structuralRows = await openResultTable("Higher-order structural paths");
  const structuralValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const scopeRows = await openResultTable("Higher-order calculation scope");
  const scopeValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const outerRows = await openResultTable("Outer loadings");
  const outerText = compactVisibleText(await page.locator(".nd-result-table tbody").textContent());
  const resultText = compactVisibleText(await page.locator(".nd-results-workspace").textContent());
  const runDetails = await inspectCurrentRunDetails();
  evidence.checks.hocResult = {
    runId: hocRunId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    component: { rows: componentRows, headers: componentHeaders, values: componentValues },
    structural: { rows: structuralRows, values: structuralValues },
    scope: { rows: scopeRows, values: scopeValues },
    ordinaryOuterRows: outerRows,
    ordinaryOuterText: outerText,
    runDetails,
    noTechnicalIds: !/__qpls_hoc_/i.test(resultText),
    noPlaceholder: !/\bN\/A\b/i.test(resultText),
  };
  if (initialSelectedTable !== "hoc_component_relationships"
    || componentRows !== 2
    || JSON.stringify(componentHeaders) !== JSON.stringify(["Higher-order construct", "Lower-order component", "Method", "Loading", "Weight"])
    || !componentValues.every((row) => row.includes("Organizational Capability") && row.includes("Disjoint two-stage"))
    || !componentValues.some((row) => row.includes("Capability")) || !componentValues.some((row) => row.includes("Resources"))
    || structuralRows !== 1 || !structuralValues[0]?.some((value) => value.includes("Organizational Capability")) || !structuralValues[0]?.some((value) => value.includes("Performance"))
    || scopeRows !== 1 || !scopeValues[0]?.includes("Reflective-reflective disjoint two-stage")
    || !scopeValues[0]?.includes("Point estimates only in the bounded native workflow")
    || outerRows !== 3 || /Organizational Capability|__qpls_hoc_/i.test(outerText)
    || runDetails.properties.Method !== "PLS-SEM Algorithm" || !String(runDetails.properties["Method version"]).includes("pls_pm_v1")
    || !evidence.checks.hocResult.noTechnicalIds || !evidence.checks.hocResult.noPlaceholder) {
    throw new Error(`The completed HOC result did not expose the exact component/path/scope tables without technical pseudo-indicators: ${JSON.stringify(evidence.checks.hocResult)}`);
  }
  await openResultTable("Higher-order component relationships");
  await capture(hocCaptureName(104, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const hocTableTitles = [
    "Higher-order component relationships",
    "Higher-order structural paths",
    "Higher-order calculation scope",
  ];
  const expectedHocSheets = [...hocTableTitles.map((title) => title.slice(0, 31)), "Run provenance"];
  evidence.checks.hocExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    formats: (await exportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText),
    nativeXlsx: null,
  };
  if (!evidence.checks.hocExport.xlsxEnabled) throw new Error("The HOC result did not enable native XLSX export.");
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: hocExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: expectedHocSheets,
    expectedSharedStrings: [...hocTableTitles, "Run provenance", "Disjoint two-stage", "Point estimates only in the bounded native workflow"],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native HOC XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native HOC XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(hocExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(hocExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(hocExportTargetPath);
    evidence.checks.hocExport.nativeXlsx = {
      attempted: true,
      targetPath: hocExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.hocExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedHocSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine HOC XLSX did not contain every bounded result and provenance sheet: ${JSON.stringify(evidence.checks.hocExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(hocCaptureName(105, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedHigherOrderArchive(hocProjectPath, hocRunId, {
    hocId,
    componentIds: [authored.ids.capability, authored.ids.resources],
    outcomeId: authored.ids.performance,
  });
  await reloadToLauncher();
  await openRecentProject(hocProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Algorithm/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened HOC result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedComponentRows = await openResultTable("Higher-order component relationships");
  const reopenedStructuralRows = await openResultTable("Higher-order structural paths");
  const reopenedScopeRows = await openResultTable("Higher-order calculation scope");
  evidence.checks.hocSaveReopen = {
    expectedRunId: hocRunId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === hocRunId,
    componentRows: reopenedComponentRows,
    structuralRows: reopenedStructuralRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.hocSaveReopen.sameRunRestored || reopenedComponentRows !== 2 || reopenedStructuralRows !== 1 || reopenedScopeRows !== 1) {
    throw new Error(`The exact HOC model and completed result did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.hocSaveReopen)}`);
  }
  await openResultTable("Higher-order component relationships");
  await capture(hocCaptureName(106, "reopened"));
}

async function runFocusedPredictionAcceptance() {
  if (!requestedPredictionNativeExportPath) {
    throw new Error("QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH is required for focused packaged PLSpredict/CVPAT acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const predictionExportTargetPath = await validateRequestedNativeExportPath(
    requestedPredictionNativeExportPath,
    "QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH",
  );

  await seedDisposableRecentProject();
  await reloadToLauncher();
  await openDisposableRecentProject();
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const statusText = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const dataColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.predictionFixture = {
    projectPath: disposableProjectPath,
    status: statusText,
    cases: statusText.includes("128 cases") ? 128 : null,
    columns: dataColumns,
  };
  if (evidence.checks.predictionFixture.cases !== 128
    || !["x1", "x2", "y1", "y2"].every((column) => dataColumns.includes(column))) {
    throw new Error(`The focused prediction fixture did not expose the expected 128-row x/y indicator data: ${JSON.stringify(evidence.checks.predictionFixture)}`);
  }
  await capture(predictionCaptureName(90, "fixture-data"));

  evidence.checks.predictionInitialModel = await createInitialEditableModel(disposableProjectName, disposableModelName);
  await buildTwoConstructModel();
  evidence.checks.predictionModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    paths: await structuralPaths().count(),
  };
  if (JSON.stringify(evidence.checks.predictionModel) !== JSON.stringify({ constructs: 2, assignedIndicators: 4, paths: 1 })) {
    throw new Error(`The focused prediction model was not the expected two-construct, four-indicator, one-path model: ${JSON.stringify(evidence.checks.predictionModel)}`);
  }
  await capture(predictionCaptureName(91, "model"));

  const dialog = await openCalculationFromToolbar();
  const methodListbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const methodLabels = (await methodListbox.getByRole("option").locator("strong").allTextContents()).map(compactVisibleText);
  await methodListbox.getByRole("option", { name: /PLSpredict \/ CVPAT/i }).click();
  const start = dialog.getByRole("button", { name: "Start prediction", exact: true });
  const selectedPanel = dialog.locator("#nd-calculation-panel");
  const dialogContract = {
    methods: methodLabels,
    selectedMethod: compactVisibleText(await methodListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    plan: compactVisibleText(await dialog.locator("#nd-calculation-prediction-plan strong").textContent()),
    targets: compactVisibleText(await dialog.locator("#nd-calculation-prediction-targets strong").textContent()),
    benchmarks: compactVisibleText(await dialog.locator("#nd-calculation-prediction-benchmarks strong").textContent()),
    cvpat: compactVisibleText(await dialog.locator("#nd-calculation-prediction-cvpat strong").textContent()),
    seed: Number(await dialog.locator("#nd-calculation-seed").inputValue()),
    confidenceControls: await dialog.locator("#nd-calculation-confidence").count(),
    workerControls: await dialog.locator("#nd-calculation-workers").count(),
    bootstrapControls: await dialog.locator("#nd-calculation-bootstrap-samples").count(),
    permutationControls: await dialog.locator("#nd-calculation-permutations").count(),
    startEnabled: await start.isEnabled(),
    blockers: await dialog.locator(".nd-blocker li").allTextContents(),
    noLegacyClaim: !/5\s*(?:folds?|×|x)\s*3|construct-score-only|bounded paired-loss/i.test(compactVisibleText(await selectedPanel.textContent())),
  };
  evidence.checks.predictionV2Dialog = dialogContract;
  if (JSON.stringify(dialogContract.methods) !== JSON.stringify(expectedOptionLabels)
    || dialogContract.selectedMethod !== "PLSpredict / CVPAT"
    || dialogContract.plan !== "Complete cases; seeded balanced 10-fold × 10-repeat cross-validation; deterministic modulo-4 holdout retained as a secondary check"
    || !/endogenous indicators.*primary.*construct-score metrics.*supplementary/i.test(dialogContract.targets)
    || !/indicator average \(IA\).*linear model \(LM(?:, where estimable)?\)/i.test(dialogContract.benchmarks)
    || !/single fitted model versus IA\/LM benchmarks/i.test(dialogContract.cvpat)
    || !/one-sided test, 95% confidence/i.test(dialogContract.cvpat)
    || !/not a comparison of saved models/i.test(dialogContract.cvpat)
    || !Number.isInteger(dialogContract.seed) || dialogContract.seed < 0
    || dialogContract.confidenceControls !== 0 || dialogContract.workerControls !== 0
    || dialogContract.bootstrapControls !== 0 || dialogContract.permutationControls !== 0
    || !dialogContract.startEnabled || dialogContract.blockers.length !== 0 || !dialogContract.noLegacyClaim) {
    throw new Error(`The focused prediction dialog did not match the exact indicator-level seeded 10x10 PLSpredict/CVPAT contract: ${JSON.stringify(dialogContract)}`);
  }
  await capture(predictionCaptureName(92, "dialog"));

  const activeCapture = captureActiveCalculation(dialog, predictionCaptureName(93, "running"), "PLSpredict / CVPAT");
  await start.click();
  evidence.checks.predictionV2Progress = await activeCapture;
  await waitForSurface("results", 180_000);
  const selectedRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /PLSpredict \/ CVPAT/i });
  await selectedRunOption.waitFor({ state: "attached", timeout: 180_000 });
  const predictionRunId = await page.locator(".nd-run-select select").inputValue();
  if (!predictionRunId) throw new Error("The completed focused prediction run had no run identifier.");

  const predictionTreeIds = await page.locator('.nd-result-tree [role="treeitem"][data-result-tree-item-id]').evaluateAll((items) => items.map((item) => item.getAttribute("data-result-tree-item-id")));
  const requiredPredictionTreeIds = [
    "plspredict_indicator_summary",
    "cvpat_benchmark_assessment",
    "plspredict_validation_plan",
    "plspredict_construct_summary",
    "plspredict_holdout_indicator_summary",
    "plspredict_holdout_construct_summary",
    "plspredict_holdout_split",
  ];
  const initialPredictionSelection = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const indicatorRowCount = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const indicatorHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const indicatorRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  await capture(predictionCaptureName(94, "indicator-results"));

  const cvpatRowCount = await openResultTable("CVPAT benchmark assessment (single model)");
  const cvpatHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const cvpatRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const validationPlanRows = await openResultTable("Prediction validation plan");
  const validationPlanText = compactVisibleText(await page.locator(".nd-result-table tbody").textContent());
  const resultProperties = await page.locator('aside[aria-label="Result properties"] .nd-property-list').evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const runDetails = await inspectCurrentRunDetails();
  const allPredictionText = `${indicatorRows.flat().join(" ")} ${cvpatRows.flat().join(" ")} ${validationPlanText}`;
  evidence.checks.predictionV2Result = {
    runId: predictionRunId,
    runLabel: compactVisibleText(await selectedRunOption.textContent()),
    treeIds: predictionTreeIds,
    initialSelection: initialPredictionSelection,
    indicator: { rows: indicatorRowCount, headers: indicatorHeaders, values: indicatorRows },
    cvpat: { rows: cvpatRowCount, headers: cvpatHeaders, values: cvpatRows },
    validationPlan: { rows: validationPlanRows, text: validationPlanText },
    properties: resultProperties,
    runDetails,
    noPlaceholderOrLegacyClaim: !/\bN\/A\b|construct-score-only|5\s*(?:folds?|×|x)\s*3|comparison of saved models/i.test(allPredictionText),
  };
  if (!requiredPredictionTreeIds.every((id) => predictionTreeIds.includes(id))
    || initialPredictionSelection !== "plspredict_indicator_summary"
    || indicatorRowCount !== 2 || !indicatorHeaders.includes("Indicator") || !indicatorHeaders.includes("Q²_predict")
    || !indicatorHeaders.includes("PLS-SEM RMSE") || !indicatorHeaders.includes("IA RMSE") || !indicatorHeaders.includes("LM RMSE")
    || !indicatorRows.every((row) => row.includes("Construct 2") && row.some((cell) => /^y[12]$/.test(cell)))
    || cvpatRowCount !== 2 || !cvpatHeaders.includes("Mean loss difference (PLS-SEM − benchmark)")
    || !cvpatRows.some((row) => row.includes("Indicator average (IA)"))
    || !cvpatRows.some((row) => row.includes("Linear model (LM)"))
    || !cvpatRows.every((row) => row.includes("PLS-SEM loss < benchmark") && row.includes("95%"))
    || validationPlanRows !== 1 || !validationPlanText.includes("Primary repeated cross-validation")
    || !validationPlanText.includes("10") || !/sha256:[0-9a-f]{64}/.test(validationPlanText)
    || resultProperties.Method !== "PLSpredict / CVPAT" || resultProperties["Complete cases"] !== "128"
    || resultProperties.Folds !== "10" || resultProperties.Repeats !== "10"
    || resultProperties.CVPAT !== "One-sided, 95% confidence"
    || runDetails.properties.Method !== "PLSpredict / CVPAT"
    || runDetails.properties["Method version"] !== `pls_pm_v1+${predictionMethodVersion}+pls_mediation_v1+pls_assessment_v7`
    || runDetails.properties["Recorded seed"] !== String(dialogContract.seed)
    || runDetails.logEntries < 1 || !evidence.checks.predictionV2Result.noPlaceholderOrLegacyClaim) {
    throw new Error(`The focused packaged prediction result did not expose the exact indicator, two-row CVPAT, validation-plan, provenance, and no-placeholder contract: ${JSON.stringify(evidence.checks.predictionV2Result)}`);
  }
  await openResultTable("CVPAT benchmark assessment (single model)");
  await capture(predictionCaptureName(95, "cvpat-results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const exportLabels = (await exportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText);
  const expectedPredictionSheets = [
    "Indicator prediction summary (1",
    "CVPAT benchmark assessment (sin",
    "Prediction validation plan",
    "Supplementary construct-score p",
    "Secondary holdout indicator sum",
    "Secondary holdout construct-sco",
    "Secondary deterministic holdout",
    "Run provenance",
  ];
  evidence.checks.predictionV2Export = {
    formats: exportLabels,
    xlsxEnabled: await xlsxExport.isEnabled(),
    nativeXlsx: null,
  };
  if (!["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"].every((label) => exportLabels.includes(label))
    || !evidence.checks.predictionV2Export.xlsxEnabled) {
    throw new Error(`The focused prediction result did not expose all table, workbook, diagram, and print exports: ${JSON.stringify(evidence.checks.predictionV2Export)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: predictionExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: expectedPredictionSheets,
    expectedSharedStrings: [
      "Indicator prediction summary (10-fold × 10-repeat)",
      "CVPAT benchmark assessment (single model)",
      "Prediction validation plan",
      "Run provenance",
      "Indicator average (IA)",
      "Linear model (LM)",
      "Assignment digest",
      `pls_pm_v1+${predictionMethodVersion}+pls_mediation_v1+pls_assessment_v7`,
      predictionRepeatedMethodVersion,
      predictionCvpatMethodVersion,
    ],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native prediction XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native prediction XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(predictionExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(predictionExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(predictionExportTargetPath);
    evidence.checks.predictionV2Export.nativeXlsx = {
      attempted: true,
      targetPath: predictionExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.predictionV2Export.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedPredictionSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine prediction XLSX did not contain every current v2 result and provenance sheet: ${JSON.stringify(evidence.checks.predictionV2Export)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(predictionCaptureName(96, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedPredictionArchive(disposableProjectPath, predictionRunId);
  await reloadToLauncher();
  await openDisposableRecentProject();
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /PLSpredict \/ CVPAT/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened prediction option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedIndicatorRows = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const reopenedCvpatRows = await openResultTable("CVPAT benchmark assessment (single model)");
  evidence.checks.predictionV2SaveReopen = {
    expectedRunId: predictionRunId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === predictionRunId,
    indicatorRows: reopenedIndicatorRows,
    cvpatRows: reopenedCvpatRows,
    archive: savedArchive,
  };
  if (!evidence.checks.predictionV2SaveReopen.sameRunRestored || reopenedIndicatorRows !== 2 || reopenedCvpatRows !== 2) {
    throw new Error(`The focused v2 prediction run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.predictionV2SaveReopen)}`);
  }
  await capture(predictionCaptureName(97, "reopened"));
}

try {
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.runtime = await page.evaluate(() => ({
    title: document.title,
    tauriRuntime: Boolean(window.__TAURI_INTERNALS__),
    viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
    surface: document.querySelector(".nd-app")?.getAttribute("data-surface"),
  }));
  if (!evidence.checks.runtime.tauriRuntime) throw new Error("The inspected page is not running inside Tauri.");
  nativeViewportLabel = `${evidence.checks.runtime.viewport.width}x${evidence.checks.runtime.viewport.height}`;
  if (gscaOnly) {
    await runFocusedGscaAcceptance();
  } else if (cbsemOnly) {
    await runFocusedCbsemAcceptance();
  } else if (olsOnly) {
    await runFocusedOlsAcceptance();
  } else if (pcaOnly) {
    await runFocusedPcaAcceptance();
  } else if (hocOnly) {
    await runFocusedHigherOrderAcceptance();
  } else if (predictionOnly) {
    await runFocusedPredictionAcceptance();
  } else {
  if (ncaOnly) {
    await reloadToLauncher();
  } else {
    if (!mgaOnly) {
    await capture("12-tauri-native-launcher-1440x900.png");

    await openMenuItem("File", "Open Sample Project");
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").first().waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.sample = {
    project: (await page.locator(".nd-window-project").textContent())?.trim(),
    constructs: await page.locator(".react-flow__node-latent").count(),
  };
  await capture("13-tauri-native-model-1440x900.png");

  await openMenuItem("View", "Data");
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  await capture("14-tauri-native-data-1440x900.png");

  const recodeCommand = page.locator(".nd-commandbar button").filter({ hasText: /Recode Variable/i });
  if (await recodeCommand.count() !== 1) throw new Error("The Data toolbar did not expose exactly one Recode Variable command.");
  await recodeCommand.click();
  const recodeDialog = page.locator('.nd-dialog-recode-data[role="dialog"]');
  await recodeDialog.waitFor({ state: "visible", timeout: 5_000 });
  await recodeDialog.getByLabel("New indicator", { exact: true }).fill("COMP1_recode");
  await recodeDialog.getByLabel("Type", { exact: true }).selectOption("numeric");
  await recodeDialog.getByLabel("Scale", { exact: true }).selectOption("continuous");
  await recodeDialog.getByLabel("Mapping 1 source value", { exact: true }).fill("6");
  await recodeDialog.getByLabel(/Mapping 1 new value/i).fill("60");
  const createRecode = recodeDialog.getByRole("button", { name: "Create Recode", exact: true });
  evidence.checks.recodeDialog = {
    source: await recodeDialog.getByLabel("Source indicator", { exact: true }).inputValue(),
    browserOnlyNoticeCount: await recodeDialog.getByText(/Browser preview cannot write dataset versions/i).count(),
    createEnabled: await createRecode.isEnabled(),
  };
  if (!evidence.checks.recodeDialog.createEnabled) throw new Error("Native Recode remained disabled after a valid mapping was entered.");
  await capture("15-tauri-native-recode-dialog-1440x900.png");
  await createRecode.click();
  await recodeDialog.waitFor({ state: "hidden", timeout: 15_000 });
  await page.getByText("Versions (2)", { exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.recodeCompleted = {
    versionCountText: (await page.getByText("Versions (2)", { exact: true }).textContent())?.trim(),
    derivedVariableVisible: await page.getByText("COMP1_recode", { exact: true }).count(),
  };
  if (!evidence.checks.recodeCompleted.derivedVariableVisible) throw new Error("The derived COMP1_recode indicator was not visible after the native mutation completed.");
  await capture("16-tauri-native-data-version-1440x900.png");

  await openMenuItem("View", "Model");
  await waitForSurface("model");
  const calculate = page.locator(".nd-commandbar button").filter({ hasText: /^Calculate/i });
  if (await calculate.count() !== 1) throw new Error("The Model toolbar did not expose exactly one generic Calculate command.");

  const calculateMenuTrigger = page.getByRole("menuitem", { name: "Calculate", exact: true });
  await calculateMenuTrigger.click();
  const calculateMenuId = await calculateMenuTrigger.getAttribute("aria-controls");
  if (!calculateMenuId) throw new Error("The Calculate menu trigger did not reference its popup.");
  const calculateMenu = page.locator(`#${calculateMenuId}`);
  await calculateMenu.waitFor({ state: "visible", timeout: 5_000 });
  const calculateMenuLabels = (await calculateMenu.getByRole("menuitem").allTextContents()).map((label) => label.trim());
  const methodSpecificMenuLabels = calculateMenuLabels.filter((label) => /PLS Algorithm|Bootstrapp|Permutation|Randomization|Construct Prediction|Prediction|Consistent PLS|Weighted PLS|CCA|composite residual|Importance-Performance|IPMA|MGA|Multi-Group|Necessary Condition|NCA/i.test(label));
  evidence.checks.calculationCommands = {
    toolbarGenericCommands: await calculate.count(),
    calculateMenuEntries: calculateMenuLabels,
    methodSpecificMenuEntries: methodSpecificMenuLabels,
  };
  if (calculateMenuLabels.length !== 1 || methodSpecificMenuLabels.length !== 0 || !/^Calculate/i.test(calculateMenuLabels[0])) {
    throw new Error(`The Calculate menu must contain one generic command, not per-method commands: ${calculateMenuLabels.join(" | ")}`);
  }
  await calculateMenu.getByRole("menuitem").click();

  const calculationDialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await calculationDialog.waitFor({ state: "visible", timeout: 5_000 });
  const methodListbox = calculationDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const methodOptions = methodListbox.getByRole("option");
  await methodOptions.first().waitFor({ state: "visible", timeout: 10_000 });
  const optionLabels = (await methodOptions.locator("strong").allTextContents()).map((label) => label.trim());
  const pathRandomizationOption = methodListbox.getByRole("option", { name: /Structural Path Randomization/i });
  const pathRandomizationDescription = (await pathRandomizationOption.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  evidence.checks.calculationCatalog = {
    options: optionLabels,
    optionCount: optionLabels.length,
    legacyTabCount: await calculationDialog.getByRole("tab").count(),
    structuralPathRandomization: {
      optionCount: await pathRandomizationOption.count(),
      description: pathRandomizationDescription,
      singleModelFreedmanLane: /single-model Freedman(?:\u2013|-|\s)Lane randomization inference/i.test(pathRandomizationDescription),
      mentionsMgaOrMicom: /\bMGA\b|\bMICOM\b/i.test(pathRandomizationDescription),
    },
  };
  if (JSON.stringify(optionLabels) !== JSON.stringify(expectedOptionLabels)) {
    throw new Error(`The calculation browser did not expose the expected ten-method catalog: ${optionLabels.join(" | ")}`);
  }
  if (evidence.checks.calculationCatalog.legacyTabCount !== 0) throw new Error("The extracted calculation browser still exposed legacy method tabs.");
  if (evidence.checks.calculationCatalog.structuralPathRandomization.optionCount !== 1
    || !evidence.checks.calculationCatalog.structuralPathRandomization.singleModelFreedmanLane
    || evidence.checks.calculationCatalog.structuralPathRandomization.mentionsMgaOrMicom) {
    throw new Error(`Structural Path Randomization was not explicitly presented as single-model Freedman-Lane path inference distinct from MGA and MICOM: ${pathRandomizationDescription}`);
  }

  const predictionOption = calculationDialog.getByRole("option", { name: /PLSpredict \/ CVPAT/i });
  const predictionOptionText = (await predictionOption.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  await predictionOption.click();
  const startPrediction = calculationDialog.getByRole("button", { name: "Start prediction", exact: true });
  const expectedPredictionPlan = "Complete cases; seeded balanced 10-fold × 10-repeat cross-validation; deterministic modulo-4 holdout retained as a secondary check";
  evidence.checks.predictionDialog = {
    selectedMethod: (await methodListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    catalogDescription: predictionOptionText,
    startEnabled: await startPrediction.isEnabled(),
    blockers: await calculationDialog.locator(".nd-blocker li").allTextContents(),
    plan: (await calculationDialog.locator("#nd-calculation-prediction-plan strong").textContent())?.trim(),
    targets: (await calculationDialog.locator("#nd-calculation-prediction-targets strong").textContent())?.trim(),
    benchmarks: (await calculationDialog.locator("#nd-calculation-prediction-benchmarks strong").textContent())?.trim(),
    cvpatScope: (await calculationDialog.locator("#nd-calculation-prediction-cvpat strong").textContent())?.trim(),
    seedControls: await calculationDialog.locator("#nd-calculation-seed").count(),
    workerControls: await calculationDialog.locator("#nd-calculation-workers").count(),
  };
  evidence.checks.predictionDialog.truthfulBoundedLabel = /PLSpredict \/ CVPAT/i.test(evidence.checks.predictionDialog.selectedMethod ?? "")
    && /endogenous-indicator prediction/i.test(predictionOptionText)
    && /10-fold × 10-repeat/i.test(predictionOptionText)
    && evidence.checks.predictionDialog.plan === expectedPredictionPlan
    && /endogenous indicators.*primary.*construct-score metrics.*supplementary/i.test(evidence.checks.predictionDialog.targets ?? "")
    && /indicator average \(IA\).*linear model \(LM(?:, where estimable)?\)/i.test(evidence.checks.predictionDialog.benchmarks ?? "")
    && /single fitted model versus IA\/LM benchmarks/i.test(evidence.checks.predictionDialog.cvpatScope ?? "")
    && /one-sided test, 95% confidence/i.test(evidence.checks.predictionDialog.cvpatScope ?? "")
    && /not a comparison of saved models/i.test(evidence.checks.predictionDialog.cvpatScope ?? "")
    && evidence.checks.predictionDialog.seedControls === 1
    && evidence.checks.predictionDialog.workerControls === 0;
  evidence.checks.predictionDialog.expectedSampleScopeBlock = !evidence.checks.predictionDialog.startEnabled
    && evidence.checks.predictionDialog.blockers.some((blocker) => /at least 20 observations/i.test(blocker));
  if (!evidence.checks.predictionDialog.truthfulBoundedLabel) throw new Error(`Prediction did not expose the bounded indicator-level PLSpredict/CVPAT label, seeded plan, IA/LM benchmarks, and one-sided single-model scope: ${JSON.stringify(evidence.checks.predictionDialog)}`);
  if (!evidence.checks.predictionDialog.expectedSampleScopeBlock) throw new Error(`The 12-row bundled sample did not expose the expected 20-observation Prediction scope blocker: ${evidence.checks.predictionDialog.blockers.join(" | ")}`);
  await capture("17-tauri-native-prediction-dialog-1440x900.png");
  await calculationDialog.getByRole("button", { name: "Close dialog", exact: true }).click();
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");

  await seedDisposableRecentProject();
  await reloadToLauncher();
  const seededRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: disposableProjectName });
  await seededRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.disposableRecentProject = {
    visibleRows: await seededRecentRow.count(),
    pathVisible: (await seededRecentRow.textContent())?.includes(disposableProjectPath) ?? false,
    projectPath: disposableProjectPath,
  };
  if (evidence.checks.disposableRecentProject.visibleRows !== 1 || !evidence.checks.disposableRecentProject.pathVisible) {
    throw new Error("The disposable 128-row project was not exposed through one truthful visible Recent Projects row.");
  }
  await capture("18-tauri-native-seeded-recent-project-1440x900.png");

  await openDisposableRecentProject();
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const largeDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  evidence.checks.nativeMethodDataset = {
    cases: largeDatasetStatus.includes("128 cases") ? 128 : null,
    columns: await page.locator(".nd-data-table thead th").allTextContents(),
    status: largeDatasetStatus,
  };
  if (evidence.checks.nativeMethodDataset.cases !== 128 || !evidence.checks.nativeMethodDataset.columns.some((column) => column.trim() === "case_wt")) {
    throw new Error(`The visible disposable project did not load the 128-row case-weight dataset: ${JSON.stringify(evidence.checks.nativeMethodDataset)}`);
  }
  await capture("19-tauri-native-method-fixture-data-1440x900.png");

  const dataGrid = page.locator('.nd-data-table[role="grid"]');
  const dataCell = dataGrid.locator('[role="gridcell"][data-native-grid-row="8"][data-native-grid-column="1"]');
  await dataCell.click();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Control+C");
  await page.waitForFunction(() => Boolean(document.querySelector('.nd-data-grid [role="status"][aria-live="polite"]')?.textContent?.trim()), null, { timeout: 5_000 });
  const activeDataCell = dataGrid.locator('[role="gridcell"][tabindex="0"]');
  evidence.checks.dataGridKeyboard = await page.evaluate(() => {
    const grid = document.querySelector('.nd-data-table[role="grid"]');
    const active = grid?.querySelector('[role="gridcell"][tabindex="0"]');
    const viewport = grid?.closest('.nd-table-scroll');
    const pager = document.querySelector('.nd-data-pager');
    const selectedHeader = grid?.querySelector('th.selected[data-native-variable]');
    const style = active ? getComputedStyle(active) : null;
    return {
      activeCells: grid?.querySelectorAll('[role="gridcell"][tabindex="0"]').length ?? 0,
      activeRow: active?.getAttribute('data-native-grid-row') ?? null,
      activeColumn: active?.getAttribute('data-native-grid-column') ?? null,
      activeVariable: active?.getAttribute('data-native-variable') ?? null,
      selectedHeader: selectedHeader?.getAttribute('data-native-variable') ?? null,
      activeCellFocused: document.activeElement === active,
      activeBoxShadow: style?.boxShadow ?? null,
      pagerVisible: Boolean(pager && pager.getBoundingClientRect().height > 0),
      viewportOverflowX: viewport ? getComputedStyle(viewport).overflowX : null,
      pageHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    };
  });
  evidence.checks.dataGridKeyboard.copyAnnouncement = (await page.locator('.nd-data-grid [role="status"][aria-live="polite"]').textContent())?.trim() ?? "";
  if (evidence.checks.dataGridKeyboard.activeCells !== 1
    || evidence.checks.dataGridKeyboard.activeRow !== "9"
    || evidence.checks.dataGridKeyboard.activeColumn !== "2"
    || evidence.checks.dataGridKeyboard.activeVariable !== evidence.checks.dataGridKeyboard.selectedHeader
    || !evidence.checks.dataGridKeyboard.activeCellFocused
    || !evidence.checks.dataGridKeyboard.activeBoxShadow
    || evidence.checks.dataGridKeyboard.activeBoxShadow === "none"
    || !evidence.checks.dataGridKeyboard.pagerVisible
    || evidence.checks.dataGridKeyboard.viewportOverflowX !== "auto"
    || evidence.checks.dataGridKeyboard.pageHorizontalOverflow
    || !evidence.checks.dataGridKeyboard.copyAnnouncement) {
    throw new Error(`The native Data grid did not satisfy its keyboard, selection, copy, or contained-scroll contract: ${JSON.stringify(evidence.checks.dataGridKeyboard)}`);
  }
  await activeDataCell.waitFor({ state: "visible", timeout: 5_000 });
  await capture("19a-tauri-native-data-grid-active-cell-1440x900.png");

  // A CLI-imported dataset truthfully starts without a canonical model. Create
  // the first editable model through the same Project Explorer flow exposed to
  // desktop users before entering the model workbench.
  evidence.checks.initialEditableModelCreation = await createInitialEditableModel(disposableProjectName, disposableModelName);
  await buildTwoConstructModel();
  evidence.checks.visibleModelBuild = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    unassignedCaseWeight: await page.locator(".nd-variable-item").filter({ hasText: /^case_wt$/ }).evaluate((element) => !element.classList.contains("assigned")),
  };
  if (evidence.checks.visibleModelBuild.constructs !== 2
    || evidence.checks.visibleModelBuild.assignedIndicators !== 4
    || evidence.checks.visibleModelBuild.structuralPaths !== 1
    || !evidence.checks.visibleModelBuild.unassignedCaseWeight) {
    throw new Error(`The visible Model workflow did not create the expected x1/x2 -> y1/y2 model: ${JSON.stringify(evidence.checks.visibleModelBuild)}`);
  }
  await capture("20-tauri-native-method-fixture-model-built-1440x900.png");

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  await reloadToLauncher();
  await openDisposableRecentProject();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").nth(1).waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.projectSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    projectPath: disposableProjectPath,
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    cases: ((await page.locator(".nd-statusbar").textContent()) ?? "").includes("128 cases") ? 128 : null,
  };
  if (evidence.checks.projectSaveReopen.constructs !== 2
    || evidence.checks.projectSaveReopen.assignedIndicators !== 4
    || evidence.checks.projectSaveReopen.structuralPaths !== 1
    || evidence.checks.projectSaveReopen.cases !== 128) {
    throw new Error(`The visible save/reload/recent-row workflow did not restore the native method project: ${JSON.stringify(evidence.checks.projectSaveReopen)}`);
  }
  await capture("21-tauri-native-method-fixture-model-reopened-1440x900.png");

  const plscDialog = await openCalculationFromToolbar();
  const plscListbox = plscDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  if (await plscListbox.getByRole("option").count() !== 10) throw new Error("The reopened calculation browser did not retain its ten-method catalog.");
  await plscListbox.getByRole("option", { name: /Consistent PLS/i }).click();
  const startPlsc = plscDialog.getByRole("button", { name: "Start consistent PLS", exact: true });
  evidence.checks.plscDialog = {
    selectedMethod: (await plscListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startPlsc.isEnabled(),
    blockers: await plscDialog.locator(".nd-blocker li").allTextContents(),
    scope: (await plscDialog.locator(".nd-setting-note.wide").filter({ hasText: /Validated scope/i }).locator("strong").textContent())?.trim(),
  };
  if (!evidence.checks.plscDialog.startEnabled) throw new Error(`Native Consistent PLS was blocked on the 128-row project: ${evidence.checks.plscDialog.blockers.join(" | ")}`);
  await capture("22-tauri-native-plsc-dialog-1440x900.png");

  const plscProgressCapture = captureActiveCalculation(
    plscDialog,
    "23-tauri-native-running-plsc-1440x900.png",
    "Consistent PLS",
  );
  await startPlsc.click();
  evidence.checks.plscProgress = await plscProgressCapture;

  await waitForSurface("results", 120_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /Consistent PLS/i }).waitFor({ state: "attached", timeout: 120_000 });
  const plscRunId = await page.locator(".nd-run-select select").inputValue();
  const plscRunLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim();
  const plscReliabilityRows = await openResultTable("PLSc correction reliability");
  const plscReliabilityText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  const plscCorrelationRows = await openResultTable("PLSc construct correlations");
  const plscCorrelationText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.plscResult = {
    runId: plscRunId,
    runLabel: plscRunLabel,
    selectedItem: (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.trim(),
    reliabilityRows: plscReliabilityRows,
    correlationRows: plscCorrelationRows,
    reliabilityText: plscReliabilityText,
    correlationText: plscCorrelationText,
    recordedSeedLabel: await page.getByText("Recorded seed", { exact: true }).count(),
  };
  if (!/Consistent PLS/i.test(plscRunLabel ?? "")) throw new Error(`The selected completed run was not Consistent PLS: ${plscRunLabel ?? "missing label"}`);
  if (!plscReliabilityRows || !plscCorrelationRows) throw new Error("The completed PLSc result did not expose non-empty correction reliability and construct-correlation tables.");
  if (![plscReliabilityText, plscCorrelationText].every((text) => text.includes("Construct 1") && text.includes("Construct 2") && !/construct-/i.test(text))) {
    throw new Error(`The completed PLSc result exposed internal construct identifiers instead of immutable model labels: ${JSON.stringify(evidence.checks.plscResult)}`);
  }
  if (evidence.checks.plscResult.recordedSeedLabel !== 0) throw new Error("The deterministic PLSc result exposed an inapplicable Recorded seed property.");
  await capture("24-tauri-native-plsc-results-1440x900.png");

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.export = {
    formats: await exportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await xlsxExport.isEnabled(),
  };
  if (evidence.checks.export.formats < 5 || !evidence.checks.export.xlsxEnabled) throw new Error("The native Export dialog did not expose the expected enabled output formats.");
  await capture("25-tauri-native-export-dialog-1440x900.png");
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/ }).click();
  await waitForSurface("model");
  evidence.checks.returnToModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
  };
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.getByRole("heading", { name: "PLSc construct correlations", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.returnToModel.selectedRunRetained = await page.locator(".nd-run-select select").inputValue() === plscRunId;
  if (!evidence.checks.returnToModel.selectedRunRetained) throw new Error("Returning from Results to Model did not retain the completed run selection.");
  await capture("26-tauri-native-returned-plsc-results-1440x900.png");

  const wplsDialog = await openCalculationFromToolbar();
  const wplsListbox = wplsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await wplsListbox.getByRole("option", { name: /Weighted PLS/i }).click();
  const caseWeight = wplsDialog.locator("#nd-calculation-case-weight");
  await caseWeight.locator('option[value="case_wt"]').waitFor({ state: "attached", timeout: 10_000 });
  const weightOptions = await caseWeight.locator("option").evaluateAll((options) => options.map((option) => ({
    label: option.textContent?.trim() ?? "",
    value: option.value,
    disabled: option.disabled,
  })));
  if (!weightOptions.some((option) => option.value === "case_wt" && !option.disabled)) {
    throw new Error(`The positive numeric case_wt variable was not available for WPLS: ${JSON.stringify(weightOptions)}`);
  }
  await caseWeight.selectOption("case_wt");
  const startWpls = wplsDialog.getByRole("button", { name: "Start weighted PLS", exact: true });
  evidence.checks.wplsDialog = {
    selectedMethod: (await wplsListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    caseWeightColumn: await caseWeight.inputValue(),
    numericWeightOptions: weightOptions.filter((option) => option.value && !option.disabled).map((option) => option.value),
    standardized: (await wplsDialog.locator(".nd-setting-note").filter({ hasText: /Result data/i }).locator("strong").textContent())?.trim(),
    startEnabled: await startWpls.isEnabled(),
    blockers: await wplsDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.wplsDialog.startEnabled) throw new Error(`Native Weighted PLS was blocked: ${evidence.checks.wplsDialog.blockers.join(" | ")}`);
  if (!/Standardized \(fixed\)/i.test(evidence.checks.wplsDialog.standardized ?? "")) throw new Error("Weighted PLS did not expose its fixed standardized preprocessing contract.");
  await capture("27-tauri-native-wpls-dialog-1440x900.png");

  const wplsProgressCapture = captureActiveCalculation(
    wplsDialog,
    "28-tauri-native-running-wpls-1440x900.png",
    "Weighted PLS",
  );
  await startWpls.click();
  evidence.checks.wplsProgress = await wplsProgressCapture;

  await waitForSurface("results");
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /Weighted PLS/i }).waitFor({ state: "attached", timeout: 120_000 });
  const wplsRunId = await page.locator(".nd-run-select select").inputValue();
  const wplsRunLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim();
  const weightedPathRows = await openResultTable("Path coefficients");
  const weightedPathText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.wplsResult = {
    runId: wplsRunId,
    runLabel: wplsRunLabel,
    pathRows: weightedPathRows,
    pathText: weightedPathText,
  };
  if (!/Weighted PLS/i.test(wplsRunLabel ?? "")) throw new Error(`The selected completed run was not Weighted PLS: ${wplsRunLabel ?? "missing label"}`);
  if (!weightedPathRows) throw new Error("The completed Weighted PLS result contained no path-coefficient rows.");
  if (!weightedPathText.includes("Construct 1") || !weightedPathText.includes("Construct 2") || /construct-/i.test(weightedPathText)) {
    throw new Error(`The completed Weighted PLS path table exposed internal construct identifiers: ${weightedPathText}`);
  }
  await capture("29-tauri-native-wpls-results-1440x900.png");

  const resultsGrid = page.locator('.nd-result-table[role="grid"]');
  await resultsGrid.locator('[role="gridcell"][data-native-grid-row="0"][data-native-grid-column="0"]').click();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Control+C");
  await page.waitForFunction(() => Boolean(document.querySelector('.nd-result-table-view [role="status"][aria-live="polite"]')?.textContent?.trim()), null, { timeout: 5_000 });
  evidence.checks.resultsGridKeyboard = await page.evaluate(() => {
    const grid = document.querySelector('.nd-result-table[role="grid"]');
    const active = grid?.querySelector('[role="gridcell"][tabindex="0"]');
    const viewport = grid?.closest('.nd-table-scroll');
    const style = active ? getComputedStyle(active) : null;
    return {
      activeCells: grid?.querySelectorAll('[role="gridcell"][tabindex="0"]').length ?? 0,
      activeRow: active?.getAttribute('data-native-grid-row') ?? null,
      activeColumn: active?.getAttribute('data-native-grid-column') ?? null,
      activeCellFocused: document.activeElement === active,
      activeBoxShadow: style?.boxShadow ?? null,
      viewportOverflowX: viewport ? getComputedStyle(viewport).overflowX : null,
      pageHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    };
  });
  evidence.checks.resultsGridKeyboard.copyAnnouncement = (await page.locator('.nd-result-table-view [role="status"][aria-live="polite"]').textContent())?.trim() ?? "";
  if (evidence.checks.resultsGridKeyboard.activeCells !== 1
    || evidence.checks.resultsGridKeyboard.activeRow !== "0"
    || evidence.checks.resultsGridKeyboard.activeColumn !== "1"
    || !evidence.checks.resultsGridKeyboard.activeCellFocused
    || !evidence.checks.resultsGridKeyboard.activeBoxShadow
    || evidence.checks.resultsGridKeyboard.activeBoxShadow === "none"
    || evidence.checks.resultsGridKeyboard.viewportOverflowX !== "auto"
    || evidence.checks.resultsGridKeyboard.pageHorizontalOverflow
    || !evidence.checks.resultsGridKeyboard.copyAnnouncement) {
    throw new Error(`The native Results grid did not satisfy its keyboard, copy, focus, or contained-scroll contract: ${JSON.stringify(evidence.checks.resultsGridKeyboard)}`);
  }
  await capture("29a-tauri-native-results-grid-active-cell-1440x900.png");

  const wplsDiagnosticRows = await openResultTable("WPLS case-weight diagnostics");
  const wplsDiagnosticText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.wpls_weights = {
    tableId: "wpls_weights",
    title: "WPLS case-weight diagnostics",
    rows: wplsDiagnosticRows,
    caseWeightColumnVisible: /case_wt/.test(wplsDiagnosticText),
  };
  if (!wplsDiagnosticRows || !evidence.checks.wpls_weights.caseWeightColumnVisible) {
    throw new Error(`The completed WPLS result did not expose truthful wpls_weights diagnostics for case_wt: ${wplsDiagnosticText}`);
  }
  await capture("30-tauri-native-wpls-weights-1440x900.png");

  const predictionRunDialog = await openCalculationFromToolbar();
  const predictionListbox = predictionRunDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await predictionListbox.getByRole("option", { name: /PLSpredict \/ CVPAT/i }).click();
  const startLargePrediction = predictionRunDialog.getByRole("button", { name: "Start prediction", exact: true });
  evidence.checks.predictionRunnableDialog = {
    selectedMethod: (await predictionListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startLargePrediction.isEnabled(),
    blockers: await predictionRunDialog.locator(".nd-blocker li").allTextContents(),
    plan: (await predictionRunDialog.locator("#nd-calculation-prediction-plan strong").textContent())?.trim(),
    seed: Number(await predictionRunDialog.locator("#nd-calculation-seed").inputValue()),
  };
  if (!evidence.checks.predictionRunnableDialog.startEnabled) {
    throw new Error(`Native Prediction was blocked on the 128-row project: ${evidence.checks.predictionRunnableDialog.blockers.join(" | ")}`);
  }
  await capture("31-tauri-native-prediction-runnable-dialog-1440x900.png");

  const predictionProgressCapture = captureActiveCalculation(
    predictionRunDialog,
    "32-tauri-native-running-prediction-1440x900.png",
    "PLSpredict / CVPAT",
  );
  await startLargePrediction.click();
  evidence.checks.predictionProgress = await predictionProgressCapture;
  await waitForSurface("results");
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLSpredict \/ CVPAT/i }).waitFor({ state: "attached", timeout: 120_000 });
  const predictionRunLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim();
  const indicatorRows = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const indicatorText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  await capture("33-tauri-native-prediction-indicator-results-1440x900.png");
  const cvpatRows = await openResultTable("CVPAT benchmark assessment (single model)");
  const cvpatText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.predictionResult = {
    runId: await page.locator(".nd-run-select select").inputValue(),
    runLabel: predictionRunLabel,
    indicatorRows,
    cvpatRows,
    indicatorText,
    cvpatText,
    noPlaceholder: !/\bN\/A\b/i.test(`${indicatorText} ${cvpatText}`),
    singleModelScope: /Indicator average \(IA\)/i.test(cvpatText)
      && /Linear model \(LM\)/i.test(cvpatText)
      && /PLS-SEM loss < benchmark/i.test(cvpatText),
  };
  if (!/PLSpredict \/ CVPAT/i.test(predictionRunLabel ?? "") || !indicatorRows || cvpatRows !== 2
    || !evidence.checks.predictionResult.noPlaceholder || !evidence.checks.predictionResult.singleModelScope) {
    throw new Error(`The completed Prediction run did not expose genuine indicator prediction and two-row single-model CVPAT benchmark outputs: ${JSON.stringify(evidence.checks.predictionResult)}`);
  }
  if (![indicatorText, cvpatText].every((text) => !/construct-/i.test(text)) || !indicatorText.includes("Construct 2") || !/\by1\b/.test(indicatorText) || !/\by2\b/.test(indicatorText)) {
    throw new Error(`The completed Prediction tables exposed internal construct identifiers instead of immutable model labels: ${JSON.stringify(evidence.checks.predictionResult)}`);
  }
  await capture("33a-tauri-native-prediction-cvpat-results-1440x900.png");

  await page.waitForTimeout(2_500);
  const archiveFailureToasts = page.locator(".nd-toast").filter({ hasText: /Recovery save failed|project archive is invalid/i });
  evidence.checks.predictionAutosave = {
    attempted: true,
    archiveFailureToasts: await archiveFailureToasts.allTextContents(),
  };
  if (evidence.checks.predictionAutosave.archiveFailureToasts.length) {
    throw new Error(`Completed Prediction could not be recovery-saved: ${evidence.checks.predictionAutosave.archiveFailureToasts.join(" | ")}`);
  }

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).waitFor({ state: "visible", timeout: 15_000 });
  await reloadToLauncher();
  await openDisposableRecentProject();
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-results-nav").waitFor({ state: "visible", timeout: 15_000 });
  await page.locator(".nd-run-select select option").first().waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunOptions = await page.locator(".nd-run-select select option").allTextContents();
  evidence.checks.completedResultsSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    runOptions: reopenedRunOptions.map((label) => label.trim()),
    hasPlsc: reopenedRunOptions.some((label) => /Consistent PLS/i.test(label)),
    hasWpls: reopenedRunOptions.some((label) => /Weighted PLS/i.test(label)),
    hasPrediction: reopenedRunOptions.some((label) => /PLSpredict \/ CVPAT/i.test(label)),
  };
  if (!evidence.checks.completedResultsSaveReopen.hasPlsc
    || !evidence.checks.completedResultsSaveReopen.hasWpls
    || !evidence.checks.completedResultsSaveReopen.hasPrediction) {
    throw new Error(`Completed method results did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.completedResultsSaveReopen)}`);
  }
  const reopenedPredictionOption = page.locator(".nd-run-select select option").filter({ hasText: /PLSpredict \/ CVPAT/i }).first();
  await reopenedPredictionOption.waitFor({ state: "attached", timeout: 15_000 });
  await page.locator(".nd-run-select select").selectOption(await reopenedPredictionOption.getAttribute("value"));
  const reopenedPredictionRows = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const reopenedCvpatRows = await openResultTable("CVPAT benchmark assessment (single model)");
  if (!reopenedPredictionRows || reopenedCvpatRows !== 2) throw new Error("The reopened Prediction run did not retain indicator-level PLSpredict and two-row CVPAT benchmark output.");
  await capture("34-tauri-native-prediction-results-reopened-1440x900.png");

  const bootstrapDialog = await openCalculationFromToolbar();
  const bootstrapListbox = bootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await bootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  await bootstrapDialog.locator("#nd-calculation-bootstrap-samples").fill("1000");
  await bootstrapDialog.locator("#nd-calculation-studentized").selectOption("999");
  const startBootstrap = bootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  if (!await startBootstrap.isEnabled()) {
    throw new Error(`The native studentized Bootstrap acceptance job was blocked: ${(await bootstrapDialog.locator(".nd-blocker li").allTextContents()).join(" | ")}`);
  }
  const bootstrapRunning = bootstrapDialog.locator(".nd-run-progress.running");
  const bootstrapRunningWait = bootstrapRunning.waitFor({ state: "visible", timeout: 20_000 });
  await startBootstrap.click();
  await bootstrapRunningWait;
  evidence.checks.bootstrapRunning = await bootstrapRunning.evaluate((element) => ({
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
  }));
  await capture("35-tauri-native-running-bootstrap-1440x900.png");
  await bootstrapDialog.getByRole("button", { name: "Cancel calculation", exact: true }).click();
  const cancelledBootstrap = bootstrapDialog.locator(".nd-run-progress.cancelled");
  await cancelledBootstrap.waitFor({ state: "visible", timeout: 30_000 });
  evidence.checks.bootstrapCancelled = {
    status: "cancelled",
    partialRunVisible: await page.locator(".nd-run-select select option").filter({ hasText: /Bootstrapping/i }).count(),
  };
  if (evidence.checks.bootstrapCancelled.partialRunVisible !== 0) {
    throw new Error("The cancelled Bootstrap job appeared as a completed Results run.");
  }
  await capture("36-tauri-native-cancelled-bootstrap-1440x900.png");
  await bootstrapDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.locator(".nd-run-select select").selectOption(wplsRunId);
  const finalWplsRows = await openResultTable("WPLS case-weight diagnostics");
  evidence.checks.finalNativeState = {
    runId: await page.locator(".nd-run-select select").inputValue(),
    title: (await page.getByRole("heading", { name: "WPLS case-weight diagnostics", exact: true }).textContent())?.trim(),
    rows: finalWplsRows,
    status: (await page.locator(".nd-statusbar strong").textContent())?.trim(),
  };
  if (evidence.checks.finalNativeState.runId !== wplsRunId || !finalWplsRows || evidence.checks.finalNativeState.status !== "Ready") {
    throw new Error(`The final native evidence state was not the completed WPLS diagnostics: ${JSON.stringify(evidence.checks.finalNativeState)}`);
  }
  await capture("37-tauri-native-final-wpls-results-1440x900.png");

  // Project Explorer milestone. All mutations use visible native commands and
  // the disposable project already exercised above. The original model is
  // deleted only after every scientific method check is complete, so this
  // section cannot weaken or short-circuit the calculation evidence.
  const initialReportName = "WPLS Review";
  const persistedReportName = "Weighted Diagnostics Review";
  const initialSecondModelName = "Alternative Model";
  const persistedSecondModelName = "Exploratory Model";
  const saveReportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Save Report/i });
  if (await saveReportCommand.count() !== 1 || !await saveReportCommand.isEnabled()) {
    throw new Error("The completed unsaved WPLS result did not expose exactly one enabled Save Report command.");
  }
  await saveReportCommand.click();
  await submitNamedExplorerDialog("Save Report", initialReportName, "Save");

  let workspaceTree = await openWorkspaceExplorer();
  await workspaceTree.waitFor({ state: "visible", timeout: 15_000 });
  const initialModelItems = workspaceTreeItem("model");
  const originalModelName = disposableModelName;
  if (await initialModelItems.count() !== 1 || await workspaceTreeItem("model", originalModelName).count() !== 1) {
    throw new Error(`The disposable project did not begin the multi-model Explorer workflow with exactly one ${originalModelName} model; found ${(await initialModelItems.allTextContents()).map((label) => label.trim()).join(", ") || "none"}.`);
  }

  const savedReportItem = workspaceTreeItem("report", initialReportName);
  await savedReportItem.waitFor({ state: "visible", timeout: 10_000 });
  await savedReportItem.click();
  await savedReportItem.focus();
  await page.keyboard.press("F2");
  await submitNamedExplorerDialog("Rename Report", persistedReportName, "Rename");
  const renamedReportItem = workspaceTreeItem("report", persistedReportName);
  await renamedReportItem.waitFor({ state: "visible", timeout: 10_000 });

  await renamedReportItem.focus();
  await page.keyboard.press("Shift+F10");
  const reportContextMenu = page.getByRole("menu", { name: "Project item commands", exact: true });
  await reportContextMenu.waitFor({ state: "visible", timeout: 5_000 });
  const reportContextLabels = (await reportContextMenu.getByRole("menuitem").allTextContents()).map((label) => label.trim());
  evidence.checks.workspaceExplorerContextMenu = {
    commands: reportContextLabels,
    openReport: reportContextLabels.some((label) => /^Open Report/i.test(label)),
    renameReport: reportContextLabels.some((label) => /^Rename Report/i.test(label)),
    removeReport: reportContextLabels.some((label) => /^Remove Report/i.test(label)),
  };
  await page.keyboard.press("Escape");
  await reportContextMenu.waitFor({ state: "hidden", timeout: 5_000 });
  evidence.checks.workspaceExplorerContextMenu.focusRestored = await renamedReportItem.evaluate((item) => document.activeElement === item);
  if (!evidence.checks.workspaceExplorerContextMenu.openReport
    || !evidence.checks.workspaceExplorerContextMenu.renameReport
    || !evidence.checks.workspaceExplorerContextMenu.removeReport
    || !evidence.checks.workspaceExplorerContextMenu.focusRestored) {
    throw new Error(`The saved-report context menu was incomplete or did not restore tree focus: ${JSON.stringify(evidence.checks.workspaceExplorerContextMenu)}`);
  }

  await workspaceTreeItem("models", "Models").click();
  const newModelDetailCommand = page.locator(".nd-explorer-detail-actions").getByRole("button", { name: "New Model", exact: true });
  if (!await newModelDetailCommand.isEnabled()) throw new Error("The writable Models folder did not expose an enabled New Model action.");
  await newModelDetailCommand.click();
  await submitNamedExplorerDialog("New Model", initialSecondModelName, "Create");
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const createdSecondModel = {
    tab: (await page.locator(".nd-document-tab span").textContent())?.trim() ?? "",
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (createdSecondModel.tab !== initialSecondModelName || createdSecondModel.constructs !== 0 || createdSecondModel.structuralPaths !== 0) {
    throw new Error(`The newly created model was not an independent empty canvas: ${JSON.stringify(createdSecondModel)}`);
  }
  await capture("37a-tauri-native-workspace-explorer-new-empty-model-1440x900.png");

  workspaceTree = await openWorkspaceExplorer();
  const initialSecondModelItem = workspaceTreeItem("model", initialSecondModelName);
  await initialSecondModelItem.waitFor({ state: "visible", timeout: 10_000 });
  await initialSecondModelItem.click();
  await initialSecondModelItem.focus();
  await page.keyboard.press("F2");
  await submitNamedExplorerDialog("Rename Model", persistedSecondModelName, "Rename");
  const persistedSecondModelItem = workspaceTreeItem("model", persistedSecondModelName);
  await persistedSecondModelItem.waitFor({ state: "visible", timeout: 10_000 });

  const explorerTreeContract = await workspaceTree.evaluate((tree) => ({
    treeItems: tree.querySelectorAll('[role="treeitem"]').length,
    tabStops: tree.querySelectorAll('[role="treeitem"][tabindex="0"]').length,
    selectedItems: tree.querySelectorAll('[role="treeitem"][aria-selected="true"]').length,
    activeModels: tree.querySelectorAll('[role="treeitem"].active-model').length,
    labels: Array.from(tree.querySelectorAll('.nd-tree-label')).map((label) => label.textContent?.trim() ?? ""),
    levels: Array.from(tree.querySelectorAll('[role="treeitem"]')).map((item) => item.getAttribute("aria-level")),
  }));
  evidence.checks.workspaceExplorerCatalog = {
    originalModelName,
    secondModelName: persistedSecondModelName,
    reportName: persistedReportName,
    ...explorerTreeContract,
  };
  if (explorerTreeContract.tabStops !== 1
    || explorerTreeContract.selectedItems !== 1
    || explorerTreeContract.activeModels !== 1
    || ![path.basename(fixtureCsvPath), "Models", "Reports", originalModelName, persistedSecondModelName, persistedReportName]
      .every((label) => explorerTreeContract.labels.includes(label))) {
    throw new Error(`The two-model Project Explorer did not satisfy its compact tree contract: ${JSON.stringify(evidence.checks.workspaceExplorerCatalog)}`);
  }
  await capture("37b-tauri-native-workspace-explorer-two-models-report-1440x900.png");

  const originalModelItem = workspaceTreeItem("model", originalModelName);
  await originalModelItem.dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").first().waitFor({ state: "visible", timeout: 15_000 });
  const originalPresentationBeforeSave = {
    tab: (await page.locator(".nd-document-tab span").textContent())?.trim() ?? "",
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  workspaceTree = await openWorkspaceExplorer();
  await workspaceTreeItem("model", persistedSecondModelName).dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const secondPresentationBeforeSave = {
    tab: (await page.locator(".nd-document-tab span").textContent())?.trim() ?? "",
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (originalPresentationBeforeSave.tab !== originalModelName
    || originalPresentationBeforeSave.constructs !== 2
    || originalPresentationBeforeSave.structuralPaths !== 1
    || secondPresentationBeforeSave.tab !== persistedSecondModelName
    || secondPresentationBeforeSave.constructs !== 0
    || secondPresentationBeforeSave.structuralPaths !== 0) {
    throw new Error(`Switching models did not restore two distinct live presentations: ${JSON.stringify({ originalPresentationBeforeSave, secondPresentationBeforeSave })}`);
  }

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await reloadToLauncher();
  await openDisposableRecentProject();
  workspaceTree = await openWorkspaceExplorer();
  await workspaceTreeItem("model", originalModelName).waitFor({ state: "visible", timeout: 15_000 });
  await workspaceTreeItem("model", persistedSecondModelName).waitFor({ state: "visible", timeout: 15_000 });
  await workspaceTreeItem("report", persistedReportName).waitFor({ state: "visible", timeout: 15_000 });

  await workspaceTreeItem("model", originalModelName).dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").first().waitFor({ state: "visible", timeout: 15_000 });
  const originalPresentationAfterReopen = {
    tab: (await page.locator(".nd-document-tab span").textContent())?.trim() ?? "",
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  workspaceTree = await openWorkspaceExplorer();
  await workspaceTreeItem("model", persistedSecondModelName).dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const secondPresentationAfterReopen = {
    tab: (await page.locator(".nd-document-tab span").textContent())?.trim() ?? "",
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  evidence.checks.workspaceExplorerSaveReopen = {
    reopenedThroughRecentProjectRow: true,
    models: [originalModelName, persistedSecondModelName],
    report: persistedReportName,
    originalPresentationBeforeSave,
    secondPresentationBeforeSave,
    originalPresentationAfterReopen,
    secondPresentationAfterReopen,
  };
  if (JSON.stringify(originalPresentationAfterReopen) !== JSON.stringify(originalPresentationBeforeSave)
    || JSON.stringify(secondPresentationAfterReopen) !== JSON.stringify(secondPresentationBeforeSave)) {
    throw new Error(`The named model catalog or distinct presentations did not survive save/reopen: ${JSON.stringify(evidence.checks.workspaceExplorerSaveReopen)}`);
  }

  workspaceTree = await openWorkspaceExplorer();
  const deletableOriginalModel = workspaceTreeItem("model", originalModelName);
  await deletableOriginalModel.click();
  await deletableOriginalModel.focus();
  await page.keyboard.press("Delete");
  const deleteModelDialog = page.getByRole("dialog", { name: "Delete Model", exact: true });
  await deleteModelDialog.waitFor({ state: "visible", timeout: 10_000 });
  const deleteModelDescription = (await deleteModelDialog.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  if (!/Completed results remain in run history/i.test(deleteModelDescription)) {
    throw new Error(`The Delete Model confirmation did not preserve-result semantics: ${deleteModelDescription}`);
  }
  await confirmExplorerRemoval("Delete Model", "Delete");
  await waitForSurface("launcher");
  await workspaceTreeItem("model", originalModelName).waitFor({ state: "hidden", timeout: 10_000 });
  await workspaceTreeItem("report", persistedReportName).waitFor({ state: "visible", timeout: 10_000 });
  const remainingModelNames = (await workspaceTreeItem("model").allTextContents()).map((label) => label.trim());
  if (remainingModelNames.length !== 1 || remainingModelNames[0] !== persistedSecondModelName) {
    throw new Error(`Deleting the historical model did not leave the independent model intact: ${JSON.stringify(remainingModelNames)}`);
  }
  await capture("37c-tauri-native-workspace-explorer-model-deleted-report-preserved-1440x900.png");

  await workspaceTreeItem("report", persistedReportName).dblclick();
  await waitForSurface("results");
  await page.locator(".nd-run-select select").waitFor({ state: "visible", timeout: 15_000 });
  const preservedReportRunId = await page.locator(".nd-run-select select").inputValue();
  const preservedReportRows = await openResultTable("WPLS case-weight diagnostics");
  const resultModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/ });
  evidence.checks.workspaceExplorerHistoricalResult = {
    deletedModel: originalModelName,
    remainingModels: remainingModelNames,
    report: persistedReportName,
    selectedRunId: preservedReportRunId,
    expectedRunId: wplsRunId,
    rows: preservedReportRows,
    editDeletedModelDisabled: await resultModelCommand.count() === 1 && !await resultModelCommand.isEnabled(),
  };
  if (preservedReportRunId !== wplsRunId
    || !preservedReportRows
    || !evidence.checks.workspaceExplorerHistoricalResult.editDeletedModelDisabled) {
    throw new Error(`Deleting a model damaged its historical result/report contract: ${JSON.stringify(evidence.checks.workspaceExplorerHistoricalResult)}`);
  }
  await capture("37d-tauri-native-workspace-explorer-historical-report-open-1440x900.png");

  workspaceTree = await openWorkspaceExplorer();
  const removableReport = workspaceTreeItem("report", persistedReportName);
  await removableReport.click();
  await removableReport.focus();
  await page.keyboard.press("Delete");
  const removeReportDialog = page.getByRole("dialog", { name: "Remove Report", exact: true });
  await removeReportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const removeReportDescription = (await removeReportDialog.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  if (!/completed result remains in run history/i.test(removeReportDescription)) {
    throw new Error(`The Remove Report confirmation did not preserve-result semantics: ${removeReportDescription}`);
  }
  await confirmExplorerRemoval("Remove Report", "Remove");
  await workspaceTreeItem("report").waitFor({ state: "hidden", timeout: 10_000 });
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const postRemovalRunOptions = await page.locator(".nd-run-select select option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.trim() ?? "",
  })));
  if (!postRemovalRunOptions.some((option) => option.value === wplsRunId)) {
    throw new Error(`Removing a report alias also removed its canonical WPLS result: ${JSON.stringify(postRemovalRunOptions)}`);
  }
  await page.locator(".nd-run-select select").selectOption(wplsRunId);
  const postRemovalRows = await openResultTable("WPLS case-weight diagnostics");
  const postRemovalSaveReportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Save Report/i });
  evidence.checks.workspaceExplorerReportRemoval = {
    aliasesRemaining: 0,
    canonicalRunRetained: postRemovalRunOptions.some((option) => option.value === wplsRunId),
    rows: postRemovalRows,
    saveReportAvailableAgain: await postRemovalSaveReportCommand.count() === 1 && await postRemovalSaveReportCommand.isEnabled(),
  };
  if (!postRemovalRows || !evidence.checks.workspaceExplorerReportRemoval.saveReportAvailableAgain) {
    throw new Error(`The canonical result was not independently usable after removing its report alias: ${JSON.stringify(evidence.checks.workspaceExplorerReportRemoval)}`);
  }
  await capture("37e-tauri-native-workspace-explorer-report-removed-result-retained-1440x900.png");

  // Genuine mediation milestone. This uses the checked-in 240-row latent
  // mediation dataset and creates every construct, indicator assignment, and
  // structural path through the same visible native workbench actions a user
  // performs. x3/m3/y3 intentionally remain unassigned.
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await seedRecentProject({
    name: mediationProjectName,
    path: mediationProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const mediationRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: mediationProjectName });
  await mediationRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mediationRecentProject = {
    visibleRows: await mediationRecentRow.count(),
    pathVisible: (await mediationRecentRow.textContent())?.includes(mediationProjectPath) ?? false,
    projectPath: mediationProjectPath,
  };
  if (evidence.checks.mediationRecentProject.visibleRows !== 1 || !evidence.checks.mediationRecentProject.pathVisible) {
    throw new Error("The disposable mediation project was not exposed through one truthful visible Recent Projects row.");
  }
  await capture(mediationCaptureName(38, "mediation-seeded-recent-project"));

  await openRecentProject(mediationProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const mediationDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  const mediationColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map((column) => column.trim());
  evidence.checks.mediationDataset = {
    cases: mediationDatasetStatus.includes("240 cases") ? 240 : null,
    columns: mediationColumns,
    status: mediationDatasetStatus,
    sourceCsv: mediationFixtureCsvPath,
  };
  const requiredMediationColumns = ["x1", "x2", "m1", "m2", "y1", "y2"];
  if (evidence.checks.mediationDataset.cases !== 240
    || requiredMediationColumns.some((column) => !mediationColumns.includes(column))) {
    throw new Error(`The visible mediation project did not load the tracked 240-row fixture: ${JSON.stringify(evidence.checks.mediationDataset)}`);
  }
  await capture(mediationCaptureName(39, "mediation-fixture-data"));

  evidence.checks.initialMediationModelCreation = await createInitialEditableModel(mediationProjectName, mediationModelName);
  await buildThreeConstructMediationModel();
  const spareIndicators = page.locator(".nd-variable-item").filter({ hasText: /^(x3|m3|y3)$/ });
  evidence.checks.visibleMediationModelBuild = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    unassignedSpareIndicators: await spareIndicators.evaluateAll((elements) => elements.filter((element) => !element.classList.contains("assigned")).map((element) => element.textContent?.trim() ?? "")),
  };
  if (evidence.checks.visibleMediationModelBuild.constructs !== 3
    || evidence.checks.visibleMediationModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleMediationModelBuild.structuralPaths !== 2
    || evidence.checks.visibleMediationModelBuild.unassignedSpareIndicators.length !== 3) {
    throw new Error(`The visible Model workflow did not create the expected x1/x2 -> m1/m2 -> y1/y2 chain: ${JSON.stringify(evidence.checks.visibleMediationModelBuild)}`);
  }
  await capture(mediationCaptureName(40, "mediation-model-built"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const mediationPlsDialog = await openCalculationFromToolbar();
  const mediationPlsListbox = mediationPlsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await mediationPlsListbox.getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const startMediationPls = mediationPlsDialog.getByRole("button", { name: "Start calculation", exact: true });
  evidence.checks.mediationPlsDialog = {
    selectedMethod: (await mediationPlsListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startMediationPls.isEnabled(),
    blockers: await mediationPlsDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.mediationPlsDialog.startEnabled) {
    throw new Error(`Native PLS was blocked on the 240-row mediation project: ${evidence.checks.mediationPlsDialog.blockers.join(" | ")}`);
  }
  await capture(mediationCaptureName(41, "mediation-pls-dialog"));
  await startMediationPls.click();

  await waitForSurface("results", 120_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Algorithm/i }).waitFor({ state: "attached", timeout: 120_000 });
  const mediationPlsRunId = await page.locator(".nd-run-select select").inputValue();
  evidence.checks.mediationPlsResult = {
    runId: mediationPlsRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: await inspectMediationResultTree({ withBootstrap: false }),
  };
  const plsSpecificText = evidence.checks.mediationPlsResult.navigation.tableText["Specific indirect effects"];
  if (!["Construct 1", "Construct 2", "Construct 3"].every((construct) => plsSpecificText.includes(construct)) || /\bN\/A\b/i.test(plsSpecificText)) {
    throw new Error(`The PLS specific-indirect table did not identify the three visible indicator-block constructs: ${plsSpecificText}`);
  }
  await capture(mediationCaptureName(42, "mediation-pls-results"));

  const mediationBootstrapDialog = await openCalculationFromToolbar();
  const mediationBootstrapListbox = mediationBootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await mediationBootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  await mediationBootstrapDialog.locator("#nd-calculation-bootstrap-samples").fill("100");
  await mediationBootstrapDialog.locator("#nd-calculation-studentized").selectOption("0");
  const startMediationBootstrap = mediationBootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  evidence.checks.mediationBootstrapDialog = {
    selectedMethod: (await mediationBootstrapListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    bootstrapSamples: await mediationBootstrapDialog.locator("#nd-calculation-bootstrap-samples").inputValue(),
    studentizedInnerSamples: await mediationBootstrapDialog.locator("#nd-calculation-studentized").inputValue(),
    startEnabled: await startMediationBootstrap.isEnabled(),
    blockers: await mediationBootstrapDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.mediationBootstrapDialog.startEnabled) {
    throw new Error(`Native Bootstrap was blocked on the 240-row mediation project: ${evidence.checks.mediationBootstrapDialog.blockers.join(" | ")}`);
  }
  await capture(mediationCaptureName(43, "mediation-bootstrap-dialog"));
  await startMediationBootstrap.click();

  await waitForSurface("results", 180_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Bootstrapping/i }).waitFor({ state: "attached", timeout: 180_000 });
  const mediationBootstrapRunId = await page.locator(".nd-run-select select").inputValue();
  evidence.checks.mediationBootstrapResult = {
    runId: mediationBootstrapRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: await inspectMediationResultTree({ withBootstrap: true }),
  };
  const bootstrapInferenceText = evidence.checks.mediationBootstrapResult.navigation.tableText[mediationBootstrapTableTitle];
  if (!bootstrapInferenceText.includes("Total indirect effect (aggregate)")
    || !bootstrapInferenceText.includes("Construct 1")
    || !bootstrapInferenceText.includes("Construct 3")
    || /\bN\/A\b/i.test(bootstrapInferenceText)) {
    throw new Error(`Aggregate mediation effects bootstrap inference did not contain the Construct 1 -> Construct 3 aggregate indirect-effect estimate: ${bootstrapInferenceText}`);
  }
  await capture(mediationCaptureName(44, "mediation-bootstrap-results"));

  const mediationExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await mediationExportCommand.click();
  const mediationExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await mediationExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const mediationXlsxExport = mediationExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await mediationXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mediationExport = {
    selectedRunId: mediationBootstrapRunId,
    formats: await mediationExportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await mediationXlsxExport.isEnabled(),
    selectedResultTable: evidence.checks.mediationBootstrapResult.navigation.selectedTable,
  };
  if (evidence.checks.mediationExport.formats < 5 || !evidence.checks.mediationExport.xlsxEnabled) {
    throw new Error("The completed mediation run did not expose the expected enabled native export formats.");
  }

  if (requestedNativeExportPath) {
    const targetPath = await validateRequestedNativeExportPath(requestedNativeExportPath);
    const nativeSaveHelper = startWindowsNativeSaveExportHelper({
      targetPath,
      windowTitle: evidence.checks.runtime.title,
    });
    let helperCompleted = false;
    try {
      const ready = await nativeSaveHelper.ready;
      evidence.checks.mediationExport.nativeXlsx = {
        attempted: true,
        targetPath,
        helper: { ready, completion: null },
        appFeedback: null,
        file: null,
      };
      if (!ready.passed || ready.event !== "ready") {
        throw new Error(`Native XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
      }

      await mediationXlsxExport.click();
      const completion = await nativeSaveHelper.completed;
      helperCompleted = true;
      evidence.checks.mediationExport.nativeXlsx.helper.completion = completion;
      if (!completion.passed) {
        throw new Error(`Native XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
      }

      const expectedFeedback = `Saved ${path.basename(targetPath)}.`;
      const feedback = mediationExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
      await feedback.waitFor({ state: "visible", timeout: 15_000 });
      const file = await fs.stat(targetPath);
      evidence.checks.mediationExport.nativeXlsx.appFeedback = (await feedback.textContent())?.trim() ?? "";
      evidence.checks.mediationExport.nativeXlsx.file = {
        path: targetPath,
        size: file.size,
        isFile: file.isFile(),
      };
      if (!file.isFile() || file.size <= 0 || evidence.checks.mediationExport.nativeXlsx.appFeedback !== expectedFeedback) {
        throw new Error(`The packaged app did not confirm the verified native XLSX export: ${JSON.stringify(evidence.checks.mediationExport.nativeXlsx)}`);
      }
    } finally {
      if (!helperCompleted) nativeSaveHelper.stop();
    }
  } else {
    evidence.checks.mediationExport.nativeXlsx = {
      attempted: false,
      reason: "QUICKPLS_NATIVE_EXPORT_PATH was not set; the harness retained the enabled native XLSX UI assertion without opening the Windows Save dialog.",
    };
  }
  await capture(mediationCaptureName(45, "mediation-export-dialog"));
  await mediationExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  await reloadToLauncher();
  await openRecentProject(mediationProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select option").first().waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMediationRuns = await page.locator(".nd-run-select select option").allTextContents();
  const reopenedMediationBootstrapOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Bootstrapping/i }).first();
  await reopenedMediationBootstrapOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMediationBootstrapRunId = await reopenedMediationBootstrapOption.getAttribute("value");
  if (!reopenedMediationBootstrapRunId) throw new Error("The reopened mediation Bootstrap run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedMediationBootstrapRunId);
  const reopenedMediationNavigation = await inspectMediationResultTree({ withBootstrap: true });

  await openMenuItem("View", "Edit Model");
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").nth(2).waitFor({ state: "visible", timeout: 15_000 });
  const reopenedMediationModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
  };
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select").selectOption(reopenedMediationBootstrapRunId);
  const reopenedMediationFinalNavigation = await inspectMediationResultTree({ withBootstrap: true });
  evidence.checks.mediationSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    runOptions: reopenedMediationRuns.map((label) => label.trim()),
    hasPlsAlgorithm: reopenedMediationRuns.some((label) => /PLS-SEM Algorithm/i.test(label)),
    hasBootstrap: reopenedMediationRuns.some((label) => /PLS-SEM Bootstrapping/i.test(label)),
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedBootstrapRunId: mediationBootstrapRunId,
    model: reopenedMediationModel,
    navigation: reopenedMediationNavigation,
    finalNavigation: reopenedMediationFinalNavigation,
  };
  if (!evidence.checks.mediationSaveReopen.hasPlsAlgorithm
    || !evidence.checks.mediationSaveReopen.hasBootstrap
    || evidence.checks.mediationSaveReopen.selectedRunId !== mediationBootstrapRunId
    || reopenedMediationModel.constructs !== 3
    || reopenedMediationModel.assignedIndicators !== 6
    || reopenedMediationModel.structuralPaths !== 2) {
    throw new Error(`The mediation model, runs, or native results did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.mediationSaveReopen)}`);
  }
  await capture(mediationCaptureName(46, "mediation-results-reopened"));

  // Genuine moderation milestone. The checked-in 120-row fixture is imported
  // into its own disposable project. Every construct, assignment, relationship,
  // and moderating effect is created through the visible native workbench.
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await seedRecentProject({
    name: moderationProjectName,
    path: moderationProjectPath,
    openedAt: "2026-08-11T00:05:00.000Z",
  });
  await reloadToLauncher();
  const moderationRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: moderationProjectName });
  await moderationRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.moderationRecentProject = {
    visibleRows: await moderationRecentRow.count(),
    pathVisible: (await moderationRecentRow.textContent())?.includes(moderationProjectPath) ?? false,
    projectPath: moderationProjectPath,
  };
  if (evidence.checks.moderationRecentProject.visibleRows !== 1 || !evidence.checks.moderationRecentProject.pathVisible) {
    throw new Error("The disposable moderation project was not exposed through one truthful visible Recent Projects row.");
  }
  await capture(moderationCaptureName(47, "moderation-seeded-recent-project"));

  await openRecentProject(moderationProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const moderationDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  const moderationColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map((column) => column.trim());
  evidence.checks.moderationDataset = {
    cases: moderationDatasetStatus.includes("120 cases") ? 120 : null,
    columns: moderationColumns,
    status: moderationDatasetStatus,
    sourceCsv: moderationFixtureCsvPath,
  };
  if (evidence.checks.moderationDataset.cases !== 120 || ["x", "m", "y"].some((column) => !moderationColumns.includes(column))) {
    throw new Error(`The visible moderation project did not load the tracked 120-row x/m/y fixture: ${JSON.stringify(evidence.checks.moderationDataset)}`);
  }
  await capture(moderationCaptureName(48, "moderation-fixture-data"));

  evidence.checks.initialModerationModelCreation = await createInitialEditableModel(moderationProjectName, moderationModelName);
  const moderationModel = await buildThreeConstructModerationModel();
  evidence.checks.visibleModerationBaseModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.replace(/\s+/g, " ").trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    basePathId: moderationModel.basePathId,
    nodeIds: { x: moderationModel.xId, moderator: moderationModel.mId, outcome: moderationModel.yId },
  };
  if (evidence.checks.visibleModerationBaseModel.constructs !== 3
    || evidence.checks.visibleModerationBaseModel.assignedIndicators !== 3
    || evidence.checks.visibleModerationBaseModel.structuralPaths !== 1
    || !["X", "M", "Y"].every((label) => evidence.checks.visibleModerationBaseModel.constructLabels.some((value) => value.includes(label)))) {
    throw new Error(`The visible Model workflow did not create the named X, M, and Y base model with X -> Y: ${JSON.stringify(evidence.checks.visibleModerationBaseModel)}`);
  }
  await capture(moderationCaptureName(49, "moderation-base-model"));

  const basePath = page.locator(`.react-flow__edge[data-id="${moderationModel.basePathId}"]`);
  await selectVisibleStructuralPath(basePath);
  const moderatingEffectCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Moderating Effect/i });
  if (await moderatingEffectCommand.count() !== 1 || !await moderatingEffectCommand.isEnabled()) {
    throw new Error("Selecting the eligible X-to-Y relationship did not enable exactly one Moderating Effect command.");
  }
  await moderatingEffectCommand.click();
  const moderationDialog = page.locator('.nd-dialog-moderation[role="dialog"]');
  await moderationDialog.waitFor({ state: "visible", timeout: 10_000 });
  const relationshipSelect = moderationDialog.locator("#nd-moderation-relationship");
  const moderatorSelect = moderationDialog.locator("#nd-moderation-moderator");
  const createModeratingEffect = moderationDialog.getByRole("button", { name: "Create moderating effect", exact: true });
  evidence.checks.moderationDialog = {
    relationshipId: await relationshipSelect.inputValue(),
    relationshipLabel: (await relationshipSelect.locator("option:checked").textContent())?.replace(/\s+/g, " ").trim() ?? "",
    moderatorId: await moderatorSelect.inputValue(),
    moderatorLabel: (await moderatorSelect.locator("option:checked").textContent())?.trim() ?? "",
    methodSummary: (await moderationDialog.locator(".nd-moderation-summary").textContent())?.replace(/\s+/g, " ").trim() ?? "",
    automaticMainEffectDisclosure: (await moderationDialog.locator(".nd-dialog-note").textContent())?.replace(/\s+/g, " ").trim() ?? "",
    createEnabled: await createModeratingEffect.isEnabled(),
  };
  if (evidence.checks.moderationDialog.relationshipId !== moderationModel.basePathId
    || !/X/.test(evidence.checks.moderationDialog.relationshipLabel)
    || !/Y/.test(evidence.checks.moderationDialog.relationshipLabel)
    || evidence.checks.moderationDialog.moderatorId !== moderationModel.mId
    || evidence.checks.moderationDialog.moderatorLabel !== "M"
    || !/Two-stage product score/i.test(evidence.checks.moderationDialog.methodSummary)
    || !/adds the moderator(?:'|\u2019)s main-effect path to the outcome when it is missing/i.test(evidence.checks.moderationDialog.automaticMainEffectDisclosure)
    || !evidence.checks.moderationDialog.createEnabled) {
    throw new Error(`The selected-path Moderating Effect dialog did not bind X -> Y, choose M, and disclose automatic M -> Y creation: ${JSON.stringify(evidence.checks.moderationDialog)}`);
  }
  await capture(moderationCaptureName(50, "moderating-effect-dialog"));
  await createModeratingEffect.click();
  await moderationDialog.waitFor({ state: "hidden", timeout: 10_000 });

  const interactionId = `interaction-${moderationModel.xId}-${moderationModel.mId}-${moderationModel.yId}`.replace(/[^a-zA-Z0-9_-]/g, "-");
  const moderatorMainEffectId = `path-${moderationModel.mId}-${moderationModel.yId}`;
  const interactionEffectId = `path-${interactionId}-${moderationModel.yId}`;
  const moderatorMainEffect = page.locator(`.react-flow__edge[data-id="${moderatorMainEffectId}"]`);
  const interactionEffect = page.locator(`.react-flow__edge[data-id="${interactionEffectId}"]`);
  await page.locator(`.react-flow__node-latent[data-id="${interactionId}"]`).waitFor({ state: "visible", timeout: 10_000 });
  await moderatorMainEffect.waitFor({ state: "attached", timeout: 10_000 });
  await interactionEffect.waitFor({ state: "attached", timeout: 10_000 });
  const basePathProperties = await inspectVisibleStructuralPath(basePath);
  const moderatorPathProperties = await inspectVisibleStructuralPath(moderatorMainEffect);
  const interactionPathProperties = await inspectVisibleStructuralPath(interactionEffect);
  evidence.checks.visibleModerationGeneratedModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.replace(/\s+/g, " ").trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    edgeIds: await structuralPaths().evaluateAll((elements) => elements.map((element) => element.getAttribute("data-id"))),
    interactionId,
    basePath: basePathProperties,
    moderatorMainEffect: moderatorPathProperties,
    interactionEffect: interactionPathProperties,
    successToast: await page.locator(".nd-toast").filter({ hasText: /Moderating effect created/i }).count(),
  };
  if (evidence.checks.visibleModerationGeneratedModel.constructs !== 4
    || evidence.checks.visibleModerationGeneratedModel.assignedIndicators !== 3
    || evidence.checks.visibleModerationGeneratedModel.structuralPaths !== 3
    || basePathProperties.Source !== "X" || basePathProperties.Target !== "Y"
    || moderatorPathProperties.Source !== "M" || moderatorPathProperties.Target !== "Y"
    || interactionPathProperties.Source !== "X x M" || interactionPathProperties.Target !== "Y"
    || evidence.checks.visibleModerationGeneratedModel.successToast !== 1) {
    throw new Error(`Creating the moderating effect did not preserve X -> Y and visibly add M -> Y plus X x M -> Y: ${JSON.stringify(evidence.checks.visibleModerationGeneratedModel)}`);
  }
  await capture(moderationCaptureName(51, "moderation-generated-model"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const moderationPlsDialog = await openCalculationFromToolbar();
  const moderationPlsListbox = moderationPlsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await moderationPlsListbox.getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const startModerationPls = moderationPlsDialog.getByRole("button", { name: "Start calculation", exact: true });
  evidence.checks.moderationPlsDialog = {
    selectedMethod: (await moderationPlsListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startModerationPls.isEnabled(),
    blockers: await moderationPlsDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.moderationPlsDialog.startEnabled) {
    throw new Error(`Native PLS was blocked on the visible 120-row moderation model: ${evidence.checks.moderationPlsDialog.blockers.join(" | ")}`);
  }
  await capture(moderationCaptureName(52, "moderation-pls-dialog"));
  await startModerationPls.click();

  await waitForSurface("results", 120_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Algorithm/i }).waitFor({ state: "attached", timeout: 120_000 });
  const moderationPlsRunId = await page.locator(".nd-run-select select").inputValue();
  const moderationPlsNavigation = await inspectModerationResultTree({ withBootstrap: false });
  evidence.checks.moderationPlsResult = {
    runId: moderationPlsRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: moderationPlsNavigation,
  };
  const moderationEffectsText = moderationPlsNavigation.tableText["Moderation effects"];
  const simpleSlopeText = moderationPlsNavigation.tableText["Simple slope analysis"];
  if (!/Interaction effect/i.test(moderationEffectsText)
    || !["X", "M", "Y"].every((label) => moderationEffectsText.includes(label))
    || moderationPlsNavigation.rowCounts["Simple slope analysis"] < 3
    || !["-1", "0", "1"].every((score) => simpleSlopeText.includes(score))) {
    throw new Error(`The completed moderation PLS run did not expose the expected effect roles and -1/0/+1 simple slopes: ${JSON.stringify(evidence.checks.moderationPlsResult)}`);
  }
  await openResultTable("Moderation effects");
  await capture(moderationCaptureName(53, "moderation-pls-effects"));
  await openResultTable("Simple slope analysis");
  await capture(moderationCaptureName(54, "moderation-pls-conditional-effect-plot"));

  const moderationBootstrapDialog = await openCalculationFromToolbar();
  const moderationBootstrapListbox = moderationBootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await moderationBootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  const moderationBootstrapSamples = moderationBootstrapDialog.locator("#nd-calculation-bootstrap-samples");
  const moderationBootstrapMinimum = await moderationBootstrapSamples.getAttribute("min");
  await moderationBootstrapSamples.fill(moderationBootstrapMinimum ?? "100");
  await moderationBootstrapDialog.locator("#nd-calculation-studentized").selectOption("0");
  const startModerationBootstrap = moderationBootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  evidence.checks.moderationBootstrapDialog = {
    selectedMethod: (await moderationBootstrapListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    bootstrapSamples: await moderationBootstrapSamples.inputValue(),
    inputMinimum: moderationBootstrapMinimum,
    minimumValidBootstrapSamples: 100,
    studentizedInnerSamples: await moderationBootstrapDialog.locator("#nd-calculation-studentized").inputValue(),
    startEnabled: await startModerationBootstrap.isEnabled(),
    blockers: await moderationBootstrapDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.moderationBootstrapDialog.startEnabled
    || evidence.checks.moderationBootstrapDialog.inputMinimum !== String(evidence.checks.moderationBootstrapDialog.minimumValidBootstrapSamples)
    || evidence.checks.moderationBootstrapDialog.bootstrapSamples !== String(evidence.checks.moderationBootstrapDialog.minimumValidBootstrapSamples)) {
    throw new Error(`Native Bootstrap was blocked at the minimum valid 100 samples on the moderation model: ${JSON.stringify(evidence.checks.moderationBootstrapDialog)}`);
  }
  await capture(moderationCaptureName(55, "moderation-bootstrap-dialog-minimum-samples"));
  await startModerationBootstrap.click();

  await waitForSurface("results", 180_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Bootstrapping/i }).waitFor({ state: "attached", timeout: 180_000 });
  const moderationBootstrapRunId = await page.locator(".nd-run-select select").inputValue();
  const moderationBootstrapNavigation = await inspectModerationResultTree({ withBootstrap: true });
  evidence.checks.moderationBootstrapResult = {
    runId: moderationBootstrapRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: moderationBootstrapNavigation,
  };
  const interactionInferenceText = moderationBootstrapNavigation.tableText[moderationBootstrapTableTitle];
  if (!["X", "M", "Y"].every((label) => interactionInferenceText.includes(label))
    || moderationBootstrapNavigation.rowCounts[moderationBootstrapTableTitle] < 1
    || /\bN\/A\b/i.test(interactionInferenceText)) {
    throw new Error(`Interaction effect bootstrap inference did not contain the genuine X x M -> Y estimate: ${interactionInferenceText}`);
  }
  await openResultTable(moderationBootstrapTableTitle);
  await capture(moderationCaptureName(56, "moderation-bootstrap-interaction-inference"));
  await openResultTable("Simple slope analysis");
  await capture(moderationCaptureName(57, "moderation-bootstrap-conditional-effect-plot"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  await reloadToLauncher();
  await openRecentProject(moderationProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select option").first().waitFor({ state: "attached", timeout: 15_000 });
  const reopenedModerationRuns = await page.locator(".nd-run-select select option").allTextContents();
  const reopenedModerationBootstrapOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Bootstrapping/i }).first();
  await reopenedModerationBootstrapOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedModerationBootstrapRunId = await reopenedModerationBootstrapOption.getAttribute("value");
  if (!reopenedModerationBootstrapRunId) throw new Error("The reopened moderation Bootstrap run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedModerationBootstrapRunId);
  const reopenedModerationNavigation = await inspectModerationResultTree({ withBootstrap: true });

  await openMenuItem("View", "Edit Model");
  await waitForSurface("model");
  await page.locator(`.react-flow__node-latent[data-id="${interactionId}"]`).waitFor({ state: "visible", timeout: 15_000 });
  const reopenedInteraction = page.locator(`.react-flow__node-latent[data-id="${interactionId}"]`);
  await reopenedInteraction.click();
  const reopenedInteractionProperties = await page.locator('aside[aria-label="Model properties"] .nd-property-list').first().evaluate((element) => Object.fromEntries(Array.from(element.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const reopenedModerationModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    interactionProperties: reopenedInteractionProperties,
  };
  await capture(moderationCaptureName(58, "moderation-model-reopened"));

  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select").selectOption(reopenedModerationBootstrapRunId);
  const reopenedModerationFinalNavigation = await inspectModerationResultTree({ withBootstrap: true });
  evidence.checks.moderationSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    runOptions: reopenedModerationRuns.map((label) => label.trim()),
    hasPlsAlgorithm: reopenedModerationRuns.some((label) => /PLS-SEM Algorithm/i.test(label)),
    hasBootstrap: reopenedModerationRuns.some((label) => /PLS-SEM Bootstrapping/i.test(label)),
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedBootstrapRunId: moderationBootstrapRunId,
    model: reopenedModerationModel,
    navigation: reopenedModerationNavigation,
    finalNavigation: reopenedModerationFinalNavigation,
  };
  if (!evidence.checks.moderationSaveReopen.hasPlsAlgorithm
    || !evidence.checks.moderationSaveReopen.hasBootstrap
    || evidence.checks.moderationSaveReopen.selectedRunId !== moderationBootstrapRunId
    || reopenedModerationBootstrapRunId !== moderationBootstrapRunId
    || reopenedModerationModel.constructs !== 4
    || reopenedModerationModel.assignedIndicators !== 3
    || reopenedModerationModel.structuralPaths !== 3
    || reopenedInteractionProperties.Predictor !== "X"
    || reopenedInteractionProperties.Moderator !== "M"
    || reopenedInteractionProperties.Outcome !== "Y"
    || reopenedInteractionProperties.Method !== "Two-stage product score") {
    throw new Error(`The moderation model, generated relationships, completed runs, or result navigation did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.moderationSaveReopen)}`);
  }
  await openResultTable(moderationBootstrapTableTitle);
  await capture(moderationCaptureName(59, "moderation-results-reopened"));
    }

  // Genuine two-group permutation MGA milestone. The deterministic 180-row
  // fixture is imported through the CLI, opened through Recent Projects, and
  // configured only through the visible Data workspace grouping command.
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await seedRecentProject({
    name: mgaProjectName,
    path: mgaProjectPath,
    openedAt: "2026-08-11T02:00:00.000Z",
  });
  await reloadToLauncher();
  const mgaRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: mgaProjectName });
  await mgaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mgaRecentProject = {
    visibleRows: await mgaRecentRow.count(),
    pathVisible: (await mgaRecentRow.textContent())?.includes(mgaProjectPath) ?? false,
    projectPath: mgaProjectPath,
  };
  if (evidence.checks.mgaRecentProject.visibleRows !== 1 || !evidence.checks.mgaRecentProject.pathVisible) {
    throw new Error("The disposable MGA project was not exposed through one truthful visible Recent Projects row.");
  }

  await openRecentProject(mgaProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const mgaDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  const mgaColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map((column) => column.trim());
  const mgaPreviewRows = await page.locator(".nd-data-table tbody tr").count();
  evidence.checks.mgaDataset = {
    cases: mgaDatasetStatus.includes("180 cases") ? 180 : null,
    columns: mgaColumns,
    previewRows: mgaPreviewRows,
    status: mgaDatasetStatus,
    sourceCsv: mgaFixtureCsvPath,
  };
  if (evidence.checks.mgaDataset.cases !== 180
    || ["group", "x1", "x2", "z1", "z2", "y1", "y2"].some((column) => !mgaColumns.includes(column))
    || mgaPreviewRows <= 0
    || mgaPreviewRows > 100) {
    throw new Error(`The visible MGA project did not load the deterministic 180-row fixture through its bounded Data preview: ${JSON.stringify(evidence.checks.mgaDataset)}`);
  }
  await capture(mgaCaptureName(60, "data"));

  evidence.checks.initialMgaModelCreation = await createInitialEditableModel(mgaProjectName, mgaModelName);
  await buildThreeConstructMgaModel();
  const mgaGroupVariable = page.locator(".nd-variable-item").filter({ hasText: /^group$/i });
  evidence.checks.visibleMgaModelBuild = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.replace(/\s+/g, " ").trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    groupVariableVisible: await mgaGroupVariable.count(),
    groupVariableUnassigned: await mgaGroupVariable.evaluate((element) => !element.classList.contains("assigned")),
  };
  if (evidence.checks.visibleMgaModelBuild.constructs !== 3
    || evidence.checks.visibleMgaModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleMgaModelBuild.structuralPaths !== 2
    || evidence.checks.visibleMgaModelBuild.groupVariableVisible !== 1
    || !evidence.checks.visibleMgaModelBuild.groupVariableUnassigned
    || !["X", "Z", "Y"].every((label) => evidence.checks.visibleMgaModelBuild.constructLabels.some((value) => value.includes(label)))) {
    throw new Error(`The visible MGA authoring workflow did not create X[x1,x2], Z[z1,z2], Y[y1,y2] with X -> Y and Z -> Y: ${JSON.stringify(evidence.checks.visibleMgaModelBuild)}`);
  }
  await capture(mgaCaptureName(61, "model"));

  await openMenuItem("View", "Data");
  await waitForSurface("data");
  const groupVariableInData = page.locator('.nd-variable-list button[data-native-variable="group"]');
  await groupVariableInData.waitFor({ state: "visible", timeout: 10_000 });
  await groupVariableInData.click();
  const configureGroupsCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Use as Grouping Variable…$/ });
  if (await configureGroupsCommand.count() !== 1 || !await configureGroupsCommand.isEnabled()) {
    throw new Error("Selecting group in Data did not expose exactly one enabled Use as Grouping Variable… command.");
  }
  await configureGroupsCommand.click();
  const groupSetupDialog = page.getByRole("dialog", { name: "Configure Groups", exact: true });
  await groupSetupDialog.waitFor({ state: "visible", timeout: 10_000 });
  const groupColumnSelect = groupSetupDialog.locator('select[id$="-column"]');
  const groupASelect = groupSetupDialog.locator('select[id$="-a"]');
  const groupBSelect = groupSetupDialog.locator('select[id$="-b"]');
  await groupASelect.locator('option[value="A"]').waitFor({ state: "attached", timeout: 30_000 });
  await groupBSelect.locator('option[value="B"]').waitFor({ state: "attached", timeout: 30_000 });
  await groupASelect.selectOption("A");
  await groupBSelect.selectOption("B");
  const groupCounts = groupSetupDialog.locator(".nd-group-counts");
  await groupCounts.waitFor({ state: "visible", timeout: 10_000 });
  const groupCountHeaders = (await groupCounts.locator("thead th").allTextContents()).map((value) => value.trim());
  const groupCountRows = await groupCounts.locator("tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const applyGroups = groupSetupDialog.getByRole("button", { name: "Apply Groups", exact: true });
  const groupScope = (await groupSetupDialog.locator(".nd-group-scope").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const observedCount = groupCountRows.reduce((sum, row) => sum + Number(row[2] ?? 0), 0);
  evidence.checks.mgaGroupSetup = {
    command: "Use as Grouping Variable…",
    groupingVariable: await groupColumnSelect.inputValue(),
    groupA: await groupASelect.inputValue(),
    groupB: await groupBSelect.inputValue(),
    headers: groupCountHeaders,
    rows: groupCountRows,
    observedCount,
    previewRows: mgaPreviewRows,
    completeDatasetBeyondPreview: observedCount === 180 && observedCount > 100 && observedCount > mgaPreviewRows,
    scope: groupScope,
    applyEnabled: await applyGroups.isEnabled(),
  };
  if (evidence.checks.mgaGroupSetup.groupingVariable !== "group"
    || evidence.checks.mgaGroupSetup.groupA !== "A"
    || evidence.checks.mgaGroupSetup.groupB !== "B"
    || JSON.stringify(groupCountHeaders) !== JSON.stringify(["Role", "Value", "Observed", "Complete model cases"])
    || JSON.stringify(groupCountRows) !== JSON.stringify([["A", "A", "90", "90"], ["B", "B", "90", "90"]])
    || !evidence.checks.mgaGroupSetup.completeDatasetBeyondPreview
    || !/Group A\s*(?:−|-)\s*Group B/i.test(groupScope)
    || !/MICOM Step 1 confirmation/i.test(groupScope)
    || !/joint MICOM\/MGA permutation plan/i.test(groupScope)
    || !evidence.checks.mgaGroupSetup.applyEnabled) {
    throw new Error(`The native Data group setup did not prove explicit A/B selection from all 180 rows: ${JSON.stringify(evidence.checks.mgaGroupSetup)}`);
  }
  await capture(mgaCaptureName(62, "group-setup"));
  await applyGroups.click();
  await groupSetupDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.locator(".nd-toast").filter({ hasText: /Groups configured/i }).last().waitFor({ state: "visible", timeout: 10_000 });
  const editGroupsCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Groups…$/ });
  evidence.checks.mgaConfiguredData = {
    groupingMarker: await groupVariableInData.locator("small").filter({ hasText: /^Groups$/ }).count(),
    groupingClass: await groupVariableInData.evaluate((element) => element.classList.contains("grouping")),
    editCommandCount: await editGroupsCommand.count(),
    editCommandEnabled: await editGroupsCommand.isEnabled(),
  };
  if (evidence.checks.mgaConfiguredData.groupingMarker !== 1
    || !evidence.checks.mgaConfiguredData.groupingClass
    || evidence.checks.mgaConfiguredData.editCommandCount !== 1
    || !evidence.checks.mgaConfiguredData.editCommandEnabled) {
    throw new Error(`Applied A/B grouping was not reflected truthfully in the Data workspace: ${JSON.stringify(evidence.checks.mgaConfiguredData)}`);
  }
  await capture(mgaCaptureName(63, "groups-applied"));

  await openMenuItem("View", "Model");
  await waitForSurface("model");
  const mgaDialog = await openCalculationFromToolbar();
  const mgaMethodListbox = mgaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const mgaMethodNames = (await mgaMethodListbox.getByRole("option").locator("strong").allTextContents()).map((label) => label.trim());
  const expectedMgaMethodNames = [
    "PLS-SEM Algorithm",
    "Consistent PLS",
    "Weighted PLS",
    "CCA composite residual diagnostics",
    "Importance-Performance Map Analysis",
    "PLS-SEM Bootstrapping",
    "Structural Path Randomization",
    "MICOM and Two-Group Permutation MGA",
    "PLSpredict / CVPAT",
    "Necessary Condition Analysis",
  ];
  if (JSON.stringify(mgaMethodNames) !== JSON.stringify(expectedMgaMethodNames)) {
    throw new Error(`The group calculation browser did not expose exactly ten truthful methods with the joint MICOM/MGA catalog entry: ${JSON.stringify({ mgaMethodNames })}`);
  }
  await mgaMethodListbox.getByRole("option", { name: /MICOM and Two-Group Permutation MGA/i }).click();
  const mgaGroupColumn = mgaDialog.locator("#nd-calculation-group-column");
  const mgaGroupA = mgaDialog.locator("#nd-calculation-group-a");
  const mgaGroupB = mgaDialog.locator("#nd-calculation-group-b");
  const mgaPermutationInput = mgaDialog.locator("#nd-calculation-group-permutations");
  const micomConfidenceInput = mgaDialog.locator("#nd-calculation-micom-confidence");
  const micomConfiguralCheckbox = mgaDialog.locator("#nd-calculation-micom-configural");
  await mgaGroupA.locator('option[value="A"]').waitFor({ state: "attached", timeout: 30_000 });
  const startMga = mgaDialog.getByRole("button", { name: "Start group analysis", exact: true });
  const mgaMethodScope = (await mgaDialog.locator(".nd-mga-settings").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const fixedWeighting = (await mgaDialog.locator(".nd-setting-note").filter({ hasText: /^Weighting scheme/ }).locator("strong").textContent())?.trim() ?? "";
  const fixedPreprocessing = (await mgaDialog.locator(".nd-setting-note").filter({ hasText: /^Result data/ }).locator("strong").textContent())?.trim() ?? "";
  const blockersBeforeConfigural = await mgaDialog.locator(".nd-blocker li").allTextContents();
  evidence.checks.mgaCalculationDialog = {
    methods: mgaMethodNames,
    selectedMethod: (await mgaMethodListbox.getByRole("option", { selected: true }).locator("strong").textContent())?.trim() ?? "",
    groupColumn: await mgaGroupColumn.inputValue(),
    groupA: await mgaGroupA.inputValue(),
    groupB: await mgaGroupB.inputValue(),
    defaultPermutations: await mgaPermutationInput.inputValue(),
    permutationMinimum: await mgaPermutationInput.getAttribute("min"),
    permutationMaximum: await mgaPermutationInput.getAttribute("max"),
    confidence: {
      value: await micomConfidenceInput.inputValue(),
      minimum: await micomConfidenceInput.getAttribute("min"),
      maximum: await micomConfidenceInput.getAttribute("max"),
      step: await micomConfidenceInput.getAttribute("step"),
    },
    configuralConfirmation: {
      count: await micomConfiguralCheckbox.count(),
      initiallyChecked: await micomConfiguralCheckbox.isChecked(),
      blockersBefore: blockersBeforeConfigural,
      checkedAfter: null,
      blockersAfter: [],
    },
    weighting: fixedWeighting,
    preprocessing: fixedPreprocessing,
    twoTailedAMinusB: /Two-tailed; Group A\s*(?:−|-)\s*Group B/i.test(mgaMethodScope),
    automaticStepsTwoAndThree: /Step 2 composition and Step 3 pooled-score means and variances are tested with the same deterministic permutations/i.test(mgaMethodScope),
    startEnabledBeforeConfigural: await startMga.isEnabled(),
  };
  if (evidence.checks.mgaCalculationDialog.selectedMethod !== "MICOM and Two-Group Permutation MGA"
    || evidence.checks.mgaCalculationDialog.groupColumn !== "group"
    || evidence.checks.mgaCalculationDialog.groupA !== "A"
    || evidence.checks.mgaCalculationDialog.groupB !== "B"
    || evidence.checks.mgaCalculationDialog.defaultPermutations !== "5000"
    || evidence.checks.mgaCalculationDialog.permutationMinimum !== "5000"
    || evidence.checks.mgaCalculationDialog.permutationMaximum !== "10000"
    || JSON.stringify(evidence.checks.mgaCalculationDialog.confidence) !== JSON.stringify({ value: "95", minimum: "80", maximum: "99.9", step: "0.1" })
    || evidence.checks.mgaCalculationDialog.configuralConfirmation.count !== 1
    || evidence.checks.mgaCalculationDialog.configuralConfirmation.initiallyChecked !== false
    || !evidence.checks.mgaCalculationDialog.configuralConfirmation.blockersBefore.some((blocker) => /Confirm MICOM Step 1/i.test(blocker))
    || evidence.checks.mgaCalculationDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.mgaCalculationDialog.preprocessing !== "Standardized (fixed)"
    || !evidence.checks.mgaCalculationDialog.twoTailedAMinusB
    || !evidence.checks.mgaCalculationDialog.automaticStepsTwoAndThree
    || evidence.checks.mgaCalculationDialog.startEnabledBeforeConfigural) {
    throw new Error(`The MICOM/MGA calculation dialog did not retain its explicit A/B, fixed path/standardized, 5,000-permutation, and Step 1 confirmation contract: ${JSON.stringify(evidence.checks.mgaCalculationDialog)}`);
  }
  await micomConfiguralCheckbox.check();
  await page.waitForFunction(() => !Array.from(document.querySelectorAll(".nd-dialog-calculation .nd-blocker li"))
    .some((item) => /Confirm MICOM Step 1/i.test(item.textContent ?? "")), undefined, { timeout: 5_000 });
  evidence.checks.mgaCalculationDialog.configuralConfirmation.checkedAfter = await micomConfiguralCheckbox.isChecked();
  evidence.checks.mgaCalculationDialog.configuralConfirmation.blockersAfter = await mgaDialog.locator(".nd-blocker li").allTextContents();
  if (!evidence.checks.mgaCalculationDialog.configuralConfirmation.checkedAfter
    || evidence.checks.mgaCalculationDialog.configuralConfirmation.blockersAfter.some((blocker) => /Confirm MICOM Step 1/i.test(blocker))) {
    throw new Error(`MICOM Step 1 confirmation did not clear its explicit blocker: ${JSON.stringify(evidence.checks.mgaCalculationDialog.configuralConfirmation)}`);
  }
  await capture(mgaCaptureName(64, "micom-v2-dialog"));

  await mgaPermutationInput.fill(String(mgaRuntimePermutationSamples));
  if (!await startMga.isEnabled()) {
    throw new Error(`The native MGA job was blocked at ${mgaRuntimePermutationSamples} valid permutations: ${(await mgaDialog.locator(".nd-blocker li").allTextContents()).join(" | ")}`);
  }
  const mgaRunningWait = mgaDialog.locator(".nd-run-progress.running").waitFor({ state: "visible", timeout: 30_000 });
  await startMga.click();
  await mgaRunningWait;
  const cancelMga = mgaDialog.getByRole("button", { name: "Cancel calculation", exact: true });
  await cancelMga.waitFor({ state: "visible", timeout: 5_000 });
  evidence.checks.mgaRunning = await mgaDialog.locator(".nd-run-progress.running").evaluate((element) => ({
    status: "running",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
  }));
  evidence.checks.mgaRunning.cancelVisible = await cancelMga.count() === 1;
  evidence.checks.mgaRunning.cancelEnabled = await cancelMga.isEnabled();
  evidence.checks.mgaRunning.permutationSamples = mgaRuntimePermutationSamples;
  if (!evidence.checks.mgaRunning.cancelVisible
    || !evidence.checks.mgaRunning.cancelEnabled
    || evidence.checks.mgaRunning.progressMax !== "100") {
    throw new Error(`The native MGA job did not expose a genuine progress and cancellation contract: ${JSON.stringify(evidence.checks.mgaRunning)}`);
  }
  await capture(mgaCaptureName(65, "micom-v2-running"));

  await waitForSurface("results", 180_000);
  const selectedMgaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /MICOM and Two-Group Permutation MGA/i });
  await selectedMgaRunOption.waitFor({ state: "attached", timeout: 180_000 });
  const mgaRunId = await page.locator(".nd-run-select select").inputValue();
  const initialMgaSelectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.replace(/\s+/g, " ").trim() ?? "";
  if (initialMgaSelectedTable !== "MICOM invariance summary") {
    throw new Error(`The completed MICOM/MGA run did not auto-open its primary invariance summary: ${initialMgaSelectedTable || "no selected table"}`);
  }
  evidence.checks.mgaResult = {
    runId: mgaRunId,
    runLabel: (await selectedMgaRunOption.textContent())?.trim() ?? "",
    autoOpenedSurface: "results",
    autoOpenedTable: initialMgaSelectedTable,
    navigation: await inspectMgaResultTree(mgaRuntimePermutationSamples),
  };
  const mgaRunDetails = await inspectCurrentRunDetails();
  const mgaRunVersionTokens = String(mgaRunDetails.properties["Method version"] ?? "").split("+");
  evidence.checks.mgaResult.runDetails = mgaRunDetails;
  if (mgaRunDetails.properties.Method !== "MICOM and Two-Group Permutation MGA"
    || mgaRunDetails.properties.Weighting !== "path"
    || mgaRunDetails.properties.Preprocessing !== "standardized"
    || ![mgaMethodVersion, mgaPermutationMethodVersion, micomMethodVersion]
      .every((version) => mgaRunVersionTokens.filter((token) => token === version).length === 1)
    || mgaRunVersionTokens.some((token) => /(?:pls_mga_two_group|pls_mga_permutation|micom)_v1$/.test(token))
    || mgaRunDetails.logEntries < 1) {
    throw new Error(`The completed group run did not expose current v2 provenance, fixed estimation scope, and genuine stored logs: ${JSON.stringify(mgaRunDetails)}`);
  }
  await capture(mgaCaptureName(66, "micom-v2-results"));

  const mgaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await mgaExportCommand.click();
  const mgaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await mgaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const mgaXlsxExport = mgaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await mgaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const mgaReviewerPackText = (await mgaExportDialog.locator(".nd-export-list button").filter({ hasText: "Reviewer pack" }).textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const mgaPrintText = (await mgaExportDialog.locator(".nd-export-list button").filter({ hasText: "Print / PDF" }).textContent())?.replace(/\s+/g, " ").trim() ?? "";
  evidence.checks.mgaExport = {
    selectedRunId: mgaRunId,
    formats: await mgaExportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await mgaXlsxExport.isEnabled(),
    modelDiagramCount: await mgaExportDialog.getByRole("button", { name: /Model diagram/i }).count(),
    reviewerPackText: mgaReviewerPackText,
    printText: mgaPrintText,
    selectedTable: initialMgaSelectedTable,
    nativeXlsx: null,
  };
  if (evidence.checks.mgaExport.formats !== 5
    || !evidence.checks.mgaExport.xlsxEnabled
    || evidence.checks.mgaExport.modelDiagramCount !== 0
    || !/Results tables and run provenance/i.test(evidence.checks.mgaExport.reviewerPackText)
    || !/Print the selected MGA results table/i.test(evidence.checks.mgaExport.printText)) {
    throw new Error(`The completed MGA result did not expose the expected native exports: ${JSON.stringify(evidence.checks.mgaExport)}`);
  }
  if (!requestedMgaNativeExportPath) {
    throw new Error("QUICKPLS_MGA_NATIVE_EXPORT_PATH is required for authoritative packaged MICOM/MGA export acceptance; the harness will not replace a genuine export with an enabled-button assertion.");
  }
  const mgaExportTargetPath = await validateRequestedNativeExportPath(requestedMgaNativeExportPath, "QUICKPLS_MGA_NATIVE_EXPORT_PATH");
  const mgaNativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: mgaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: [
      "Two-group sample summary",
      "MICOM invariance summary",
      "Group path coefficients",
      "Group outer loadings",
      "Group outer weights",
      "Run provenance",
    ],
    expectedSharedStrings: [...mgaTableContracts.map((table) => table.title), "Run provenance"],
  });
  let mgaHelperCompleted = false;
  try {
    const ready = await mgaNativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") {
      throw new Error(`Native MICOM/MGA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    }
    await mgaXlsxExport.click();
    const completion = await mgaNativeSaveHelper.completed;
    mgaHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native MICOM/MGA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(mgaExportTargetPath)}.`;
    const feedback = mgaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(mgaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(mgaExportTargetPath);
    const micomStep3Sheets = workbookSheets.filter((sheet) => sheet.startsWith("MICOM Step 3 - equality"));
    const allSheetsDistinct = new Set(workbookSheets).size === workbookSheets.length;
    evidence.checks.mgaExport.nativeXlsx = {
      attempted: true,
      targetPath: mgaExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
      allSheetsDistinct,
      micomStep3Sheets,
    };
    if (!file.isFile() || file.size <= 0
      || evidence.checks.mgaExport.nativeXlsx.appFeedback !== expectedFeedback
      || workbookSheets.length !== mgaTableContracts.length + 1
      || !allSheetsDistinct || micomStep3Sheets.length !== 2
      || !workbookSheets.includes("Run provenance")) {
      throw new Error(`The packaged MICOM/MGA XLSX did not contain every v2 table, two distinct MICOM Step 3 sheets, and provenance: ${JSON.stringify(evidence.checks.mgaExport.nativeXlsx)}`);
    }
  } finally {
    if (!mgaHelperCompleted) mgaNativeSaveHelper.stop();
  }
  await capture(mgaCaptureName(67, "export"));
  await mgaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  const editMgaModel = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/ });
  if (await editMgaModel.count() !== 1 || !await editMgaModel.isEnabled()) {
    throw new Error("Completed MGA Results did not expose exactly one enabled Edit Model command.");
  }
  await editMgaModel.click();
  await waitForSurface("model");
  const configuredGroupInModel = page.locator(".nd-variable-item").filter({ hasText: /^group\s*Group$/i });
  evidence.checks.mgaEditModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    groupingVariableCount: await configuredGroupInModel.count(),
    groupingVariableDisabled: await configuredGroupInModel.isDisabled(),
    selectedResultRetained: true,
  };
  if (evidence.checks.mgaEditModel.constructs !== 3
    || evidence.checks.mgaEditModel.assignedIndicators !== 6
    || evidence.checks.mgaEditModel.structuralPaths !== 2
    || evidence.checks.mgaEditModel.groupingVariableCount !== 1
    || !evidence.checks.mgaEditModel.groupingVariableDisabled) {
    throw new Error(`Edit Model did not restore the same live MGA model with its grouping variable protected from indicator assignment: ${JSON.stringify(evidence.checks.mgaEditModel)}`);
  }
  await capture(mgaCaptureName(68, "edit-model"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedMgaArchive = await inspectSavedMgaArchive(mgaProjectPath, mgaRunId);
  await reloadToLauncher();
  await openRecentProject(mgaProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedMgaOption = page.locator(".nd-run-select select option").filter({ hasText: /MICOM and Two-Group Permutation MGA/i }).first();
  await reopenedMgaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMgaRunId = await reopenedMgaOption.getAttribute("value");
  if (!reopenedMgaRunId) throw new Error("The reopened MGA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedMgaRunId);
  const reopenedMgaNavigation = await inspectMgaResultTree(mgaRuntimePermutationSamples);
  evidence.checks.mgaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedRunId: mgaRunId,
    expectedSelectedTable: evidence.checks.mgaResult.navigation.selectedTable,
    runOptions: (await page.locator(".nd-run-select select option").allTextContents()).map((label) => label.trim()),
    archive: savedMgaArchive,
    navigation: reopenedMgaNavigation,
  };
  if (reopenedMgaRunId !== mgaRunId
    || evidence.checks.mgaSaveReopen.selectedRunId !== mgaRunId
    || !evidence.checks.mgaSaveReopen.runOptions.some((label) => /MICOM and Two-Group Permutation MGA/i.test(label))
    || evidence.checks.mgaSaveReopen.navigation.selectedTable !== evidence.checks.mgaResult.navigation.selectedTable
    || !evidence.checks.mgaSaveReopen.navigation.allRequiredTablesVisible
    || !evidence.checks.mgaSaveReopen.navigation.noPooledDiagram
    || !evidence.checks.mgaSaveReopen.navigation.noPlaceholderNa) {
    throw new Error(`The completed group result did not survive Ctrl+S, reload, and Recent Projects reopen: ${JSON.stringify(evidence.checks.mgaSaveReopen)}`);
  }
  await capture(mgaCaptureName(69, "micom-v2-reopen"));

  if (!mgaOnly) {
  await seedRecentProject({
    name: ccaProjectName,
    path: ccaProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const ccaRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: ccaProjectName });
  await ccaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await ccaRecentRow.count() !== 1 || !(await ccaRecentRow.textContent())?.includes(ccaProjectPath)) {
    throw new Error("The deterministic CCA project was not exposed through one truthful Recent Projects row.");
  }
  await openRecentProject(ccaProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ccaDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const ccaDataHeaders = (await page.locator(".nd-data-table thead th").allTextContents()).map((value) => value.trim());
  evidence.checks.ccaFixtureProvisioning.visibleDataset = {
    cases: ccaDatasetStatus.includes("132 cases") ? 132 : null,
    columns: ccaDataHeaders,
    status: ccaDatasetStatus,
    deterministicNonSaturatedModel: "X -> Z -> Y",
  };
  if (evidence.checks.ccaFixtureProvisioning.visibleDataset.cases !== 132
    || !["x1", "x2", "z1", "z2", "y1", "y2"].every((column) => ccaDataHeaders.includes(column))) {
    throw new Error(`The visible CCA project did not load the tracked 132-row six-indicator fixture: ${JSON.stringify(evidence.checks.ccaFixtureProvisioning.visibleDataset)}`);
  }
  await capture(ccaCaptureName(70, "fixture-data"));

  const initialCcaModel = await createInitialEditableModel(ccaProjectName, ccaModelName);
  await buildThreeConstructCcaModel();
  const ccaPaths = structuralPaths();
  const firstCcaPath = await inspectVisibleStructuralPath(ccaPaths.nth(0));
  const secondCcaPath = await inspectVisibleStructuralPath(ccaPaths.nth(1));
  evidence.checks.visibleCcaModelBuild = {
    ...initialCcaModel,
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await ccaPaths.count(),
    paths: [firstCcaPath, secondCcaPath],
    nonSaturated: true,
  };
  if (evidence.checks.visibleCcaModelBuild.constructs !== 3
    || evidence.checks.visibleCcaModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleCcaModelBuild.structuralPaths !== 2
    || firstCcaPath.Source !== "X" || firstCcaPath.Target !== "Z"
    || secondCcaPath.Source !== "Z" || secondCcaPath.Target !== "Y") {
    throw new Error(`The visible CCA authoring workflow did not produce the exact non-saturated X -> Z -> Y model: ${JSON.stringify(evidence.checks.visibleCcaModelBuild)}`);
  }
  await capture(ccaCaptureName(71, "model"));

  const ccaDialog = await openCalculationFromToolbar();
  const ccaListbox = ccaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const ccaMethods = (await ccaListbox.getByRole("option").locator("strong").allTextContents()).map((label) => label.trim());
  await ccaListbox.getByRole("option", { name: /CCA composite residual diagnostics/i }).click();
  const ccaResultData = ccaDialog.locator(".nd-setting-note").filter({ hasText: "Result data" });
  const ccaScope = ccaDialog.locator(".nd-setting-note").filter({ hasText: "Validated scope" });
  const ccaMissingData = ccaDialog.locator(".nd-setting-note").filter({ hasText: "Missing data" });
  const ccaPcaOption = ccaDialog.locator('#nd-calculation-weighting option[value="pca"]');
  const startCca = ccaDialog.getByRole("button", { name: "Start composite diagnostics", exact: true });
  const ccaSelectedPanelText = (await ccaDialog.locator("#nd-calculation-panel").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  evidence.checks.ccaCalculationDialog = {
    methods: ccaMethods,
    selectedMethod: (await ccaListbox.getByRole("option", { selected: true }).locator("strong").textContent())?.trim() ?? "",
    category: (await ccaListbox.locator("#nd-calculation-category-assessment").textContent())?.trim() ?? "",
    resultData: (await ccaResultData.locator("strong").textContent())?.trim() ?? "",
    validatedScope: (await ccaScope.locator("strong").textContent())?.trim() ?? "",
    missingData: (await ccaMissingData.locator("strong").textContent())?.trim() ?? "",
    weighting: await ccaDialog.locator("#nd-calculation-weighting").inputValue(),
    pcaDisabled: await ccaPcaOption.evaluate((option) => option.disabled),
    maximumIterations: await ccaDialog.locator("#nd-calculation-max-iterations").inputValue(),
    tolerance: await ccaDialog.locator("#nd-calculation-tolerance").inputValue(),
    unsupportedControls: await ccaDialog.locator([
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
    ].join(", ")).count(),
    startEnabled: await startCca.isEnabled(),
    blockers: await ccaDialog.locator(".nd-blocker li").allTextContents(),
    noInventedDecisionControls: !/threshold|pass\/fail|fit classification|p[- ]?value|confidence interval/i.test(ccaSelectedPanelText),
  };
  if (JSON.stringify(ccaMethods) !== JSON.stringify(expectedOptionLabels)
    || evidence.checks.ccaCalculationDialog.selectedMethod !== "CCA composite residual diagnostics"
    || evidence.checks.ccaCalculationDialog.category !== "Assessment"
    || evidence.checks.ccaCalculationDialog.resultData !== "Standardized (fixed)"
    || evidence.checks.ccaCalculationDialog.validatedScope !== "Reflective composite path model; descriptive residual diagnostics only"
    || evidence.checks.ccaCalculationDialog.missingData !== "Listwise deletion"
    || evidence.checks.ccaCalculationDialog.weighting !== "path"
    || !evidence.checks.ccaCalculationDialog.pcaDisabled
    || evidence.checks.ccaCalculationDialog.maximumIterations !== "3000"
    || evidence.checks.ccaCalculationDialog.tolerance !== "1e-7"
    || evidence.checks.ccaCalculationDialog.unsupportedControls !== 0
    || !evidence.checks.ccaCalculationDialog.startEnabled
    || evidence.checks.ccaCalculationDialog.blockers.length !== 0
    || !evidence.checks.ccaCalculationDialog.noInventedDecisionControls) {
    throw new Error(`The packaged CCA setup did not match its exact bounded ready-state contract: ${JSON.stringify(evidence.checks.ccaCalculationDialog)}`);
  }
  await capture(ccaCaptureName(72, "dialog"));

  const ccaActive = ccaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const ccaActiveWait = ccaActive.waitFor({ state: "visible", timeout: 20_000 });
  await startCca.click();
  await ccaActiveWait;
  evidence.checks.ccaRunning = await ccaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  if (!["queued", "validating", "running"].includes(evidence.checks.ccaRunning.status)
    || !evidence.checks.ccaRunning.phase
    || !evidence.checks.ccaRunning.cancelVisible) {
    throw new Error(`The CCA calculation did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.ccaRunning)}`);
  }
  await capture(ccaCaptureName(73, "running"));

  await waitForSurface("results", 120_000);
  const selectedCcaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /CCA composite residual diagnostics/i });
  await selectedCcaRunOption.waitFor({ state: "attached", timeout: 120_000 });
  const ccaRunId = await page.locator(".nd-run-select select").inputValue();
  const ccaNavigation = await inspectCcaResultTree();
  const ccaRunDetails = await inspectCurrentRunDetails();
  const ccaResultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  evidence.checks.ccaResult = {
    runId: ccaRunId,
    runLabel: (await selectedCcaRunOption.textContent())?.trim() ?? "",
    autoOpenedSurface: "results",
    autoOpenedTable: ccaNavigation.initialSelectedTable,
    methodVersion: ccaRunDetails.properties["Method version"] ?? null,
    nestedModelVersion: null,
    nestedModelLabel: ccaNavigation.nestedModelLabel,
    correlationPairs: ccaNavigation.correlationPairs,
    maximumAbsoluteResidual: ccaNavigation.maximumAbsoluteResidual,
    navigation: ccaNavigation,
    runDetails: ccaRunDetails,
    resultProperties: ccaResultProperties,
  };
  if (evidence.checks.ccaResult.methodVersion !== ccaProvenanceMethodVersion
    || ccaRunDetails.properties.Method !== "CCA composite residual diagnostics"
    || ccaRunDetails.properties.Weighting !== "path"
    || ccaRunDetails.properties.Preprocessing !== "standardized"
    || Object.prototype.hasOwnProperty.call(ccaRunDetails.properties, "Recorded seed")
    || Object.prototype.hasOwnProperty.call(ccaResultProperties, "Recorded seed")
    || ccaRunDetails.logEntries < 1) {
    throw new Error(`The completed CCA run details did not expose exact provenance and genuine stored logs: ${JSON.stringify(evidence.checks.ccaResult)}`);
  }
  await capture(ccaCaptureName(74, "results"));

  const ccaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await ccaExportCommand.click();
  const ccaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await ccaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const ccaXlsxExport = ccaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await ccaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const ccaExportLabels = (await ccaExportDialog.locator(".nd-export-list button strong").allTextContents()).map((label) => label.trim());
  evidence.checks.ccaExport = {
    selectedRunId: ccaRunId,
    formats: ccaExportLabels,
    xlsxEnabled: await ccaXlsxExport.isEnabled(),
    residualTablesIncluded: false,
    nativeXlsx: null,
  };
  if (!["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"].every((label) => ccaExportLabels.includes(label))
    || !evidence.checks.ccaExport.xlsxEnabled) {
    throw new Error(`The completed CCA result did not expose the expected enabled native exports: ${JSON.stringify(evidence.checks.ccaExport)}`);
  }
  if (!requestedCcaNativeExportPath) {
    throw new Error("QUICKPLS_CCA_NATIVE_EXPORT_PATH is required for authoritative packaged CCA export acceptance; the harness will not replace a genuine export with an enabled-button assertion.");
  }
  const ccaExportTargetPath = await validateRequestedNativeExportPath(requestedCcaNativeExportPath, "QUICKPLS_CCA_NATIVE_EXPORT_PATH");
  const ccaNativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: ccaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: ["Residual summary", "Composite residuals", "Run provenance"],
    expectedSharedStrings: ["Residual summary", "Composite residuals", "Run provenance"],
  });
  let ccaHelperCompleted = false;
  try {
    const ready = await ccaNativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") {
      throw new Error(`Native CCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    }
    await ccaXlsxExport.click();
    const completion = await ccaNativeSaveHelper.completed;
    ccaHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native CCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(ccaExportTargetPath)}.`;
    const feedback = ccaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(ccaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(ccaExportTargetPath);
    evidence.checks.ccaExport.nativeXlsx = {
      attempted: true,
      targetPath: ccaExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    evidence.checks.ccaExport.residualTablesIncluded = workbookSheets.includes("Residual summary")
      && workbookSheets.includes("Composite residuals");
    if (!file.isFile() || file.size <= 0
      || evidence.checks.ccaExport.nativeXlsx.appFeedback !== expectedFeedback
      || !evidence.checks.ccaExport.residualTablesIncluded) {
      throw new Error(`The packaged CCA XLSX did not contain both residual tables with confirmed app feedback: ${JSON.stringify(evidence.checks.ccaExport)}`);
    }
  } finally {
    if (!ccaHelperCompleted) ccaNativeSaveHelper.stop();
  }
  await capture(ccaCaptureName(75, "export"));
  await ccaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedCcaArchive = await inspectSavedCcaArchive(ccaProjectPath, ccaRunId);
  evidence.checks.ccaResult.nestedModelVersion = savedCcaArchive.nestedModelVersion;
  evidence.checks.ccaResult.archiveMaximumMatchesVisible = Math.abs(
    Number(savedCcaArchive.maximumAbsoluteResidual.toFixed(4))
      - evidence.checks.ccaResult.maximumAbsoluteResidual,
  ) <= Number.EPSILON;
  if (!evidence.checks.ccaResult.archiveMaximumMatchesVisible) {
    throw new Error(`The visible and saved CCA maximum residuals diverged: ${JSON.stringify(evidence.checks.ccaResult)}`);
  }
  await reloadToLauncher();
  await openRecentProject(ccaProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedCcaOption = page.locator(".nd-run-select select option").filter({ hasText: /CCA composite residual diagnostics/i }).first();
  await reopenedCcaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedCcaRunId = await reopenedCcaOption.getAttribute("value");
  if (!reopenedCcaRunId) throw new Error("The reopened CCA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedCcaRunId);
  const reopenedCcaNavigation = await inspectCcaResultTree();
  evidence.checks.ccaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    expectedRunId: ccaRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedCcaRunId === ccaRunId
      && await page.locator(".nd-run-select select").inputValue() === ccaRunId,
    archive: savedCcaArchive,
    navigation: reopenedCcaNavigation,
  };
  if (!evidence.checks.ccaSaveReopen.sameRunRestored
    || reopenedCcaNavigation.correlationPairs !== 3
    || reopenedCcaNavigation.nestedModelLabel !== "Recursive standardized composite path model") {
    throw new Error(`The exact CCA run and residual Results did not survive explicit save and reopen: ${JSON.stringify(evidence.checks.ccaSaveReopen)}`);
  }
  await capture(ccaCaptureName(76, "reopened"));

  await seedRecentProject({
    name: ipmaProjectName,
    path: ipmaProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const ipmaRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: ipmaProjectName });
  await ipmaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await ipmaRecentRow.count() !== 1 || !(await ipmaRecentRow.textContent())?.includes(ipmaProjectPath)) {
    throw new Error("The deterministic IPMA project was not exposed through one truthful Recent Projects row.");
  }
  await openRecentProject(ipmaProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ipmaDatasetStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const ipmaDataHeaders = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.ipmaFixtureProvisioning.visibleDataset = {
    cases: ipmaDatasetStatus.includes("80 cases") ? 80 : null,
    columns: ipmaDataHeaders,
    status: ipmaDatasetStatus,
    deterministicModel: "X -> M; Z -> M; X -> Y; Z -> Y; M -> Y; disconnected U -> V negative-control branch",
  };
  if (evidence.checks.ipmaFixtureProvisioning.visibleDataset.cases !== 80
    || !["x1", "z1", "m1", "y1", "u1", "v1"].every((column) => ipmaDataHeaders.includes(column))) {
    throw new Error(`The visible IPMA project did not load the tracked 80-row six-indicator fixture: ${JSON.stringify(evidence.checks.ipmaFixtureProvisioning.visibleDataset)}`);
  }
  await capture(ipmaCaptureName(77, "fixture-data"));

  const initialIpmaModel = await createInitialEditableModel(ipmaProjectName, ipmaModelName);
  const { constructIds: ipmaConstructIds } = await buildSixConstructIpmaModelWithDisconnectedBranch();
  const ipmaPaths = structuralPaths();
  const visibleIpmaPaths = [];
  for (let index = 0; index < 6; index += 1) visibleIpmaPaths.push(await inspectVisibleStructuralPath(ipmaPaths.nth(index)));
  const observedIpmaPairs = visibleIpmaPaths.map((pathProperties) => `${pathProperties.Source} -> ${pathProperties.Target}`);
  const expectedIpmaPairs = ["X -> M", "Z -> M", "X -> Y", "Z -> Y", "M -> Y", "U -> V"];
  evidence.checks.visibleIpmaModelBuild = {
    ...initialIpmaModel,
    constructIds: ipmaConstructIds,
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await ipmaPaths.count(),
    paths: visibleIpmaPaths,
    observedPairs: observedIpmaPairs,
  };
  if (evidence.checks.visibleIpmaModelBuild.constructs !== 6
    || evidence.checks.visibleIpmaModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleIpmaModelBuild.structuralPaths !== 6
    || JSON.stringify(observedIpmaPairs) !== JSON.stringify(expectedIpmaPairs)
    || new Set(Object.values(ipmaConstructIds)).size !== 6) {
    throw new Error(`The visible IPMA authoring workflow did not produce the exact six-construct six-path model with a disconnected U-to-V negative-control branch: ${JSON.stringify(evidence.checks.visibleIpmaModelBuild)}`);
  }
  await capture(ipmaCaptureName(78, "model"));

  const ipmaDialog = await openCalculationFromToolbar();
  const ipmaListbox = ipmaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const ipmaMethods = (await ipmaListbox.getByRole("option").locator("strong").allTextContents()).map(compactVisibleText);
  await ipmaListbox.getByRole("option", { name: /Importance-Performance Map Analysis/i }).click();
  const ipmaTarget = ipmaDialog.locator("#nd-calculation-ipma-target");
  await ipmaTarget.waitFor({ state: "visible", timeout: 10_000 });
  const ipmaTargetOptions = await ipmaTarget.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
  })));
  const yTargetOption = ipmaTargetOptions.find((option) => option.value === ipmaConstructIds.y);
  if (!yTargetOption) throw new Error(`The explicit IPMA target selector did not expose Y by immutable construct id: ${JSON.stringify({ ipmaTargetOptions, ipmaConstructIds })}`);
  await ipmaTarget.selectOption(ipmaConstructIds.y);
  const ipmaNoteValue = async (label) => compactVisibleText(await ipmaDialog.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const ipmaSelectedPanelText = compactVisibleText(await ipmaDialog.locator("#nd-calculation-panel").textContent());
  const startIpma = ipmaDialog.getByRole("button", { name: "Start importance-performance analysis", exact: true });
  evidence.checks.ipmaCalculationDialog = {
    methods: ipmaMethods,
    selectedMethod: compactVisibleText(await ipmaListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await ipmaListbox.locator("#nd-calculation-category-assessment").textContent()),
    targetPlaceholder: ipmaTargetOptions.find((option) => option.value === "")?.label ?? null,
    targetOptions: ipmaTargetOptions,
    selectedTargetId: await ipmaTarget.inputValue(),
    selectedTargetLabel: compactVisibleText(await ipmaTarget.locator("option:checked").textContent()),
    weighting: await ipmaNoteValue("Weighting scheme"),
    resultData: await ipmaNoteValue("Result data"),
    missingData: await ipmaNoteValue("Missing data"),
    reportedConstructs: await ipmaNoteValue("Reported constructs"),
    performanceScope: await ipmaNoteValue("Performance scope"),
    maximumIterations: await ipmaDialog.locator("#nd-calculation-max-iterations").inputValue(),
    tolerance: await ipmaDialog.locator("#nd-calculation-tolerance").inputValue(),
    unsupportedControls: await ipmaDialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
      "#nd-calculation-group-column",
    ].join(", ")).count(),
    startEnabled: await startIpma.isEnabled(),
    blockers: await ipmaDialog.locator(".nd-blocker li").allTextContents(),
    predecessorOnly: ipmaSelectedPanelText.includes("Direct and indirect structural predecessors only; the target and unrelated constructs are omitted"),
    observedRange: ipmaSelectedPanelText.includes("observed-range scaling of standardized composite scores; no theoretical-range correction"),
    noCipmaOrInferenceClaim: !/\bcIPMA\b|p[- ]?value|confidence interval|bootstrap|permutation|resampling inference|significance/i.test(ipmaSelectedPanelText),
  };
  if (JSON.stringify(ipmaMethods) !== JSON.stringify(expectedOptionLabels)
    || evidence.checks.ipmaCalculationDialog.selectedMethod !== "Importance-Performance Map Analysis"
    || evidence.checks.ipmaCalculationDialog.category !== "Assessment"
    || evidence.checks.ipmaCalculationDialog.targetPlaceholder !== "Select one endogenous construct"
    || evidence.checks.ipmaCalculationDialog.selectedTargetId !== ipmaConstructIds.y
    || evidence.checks.ipmaCalculationDialog.selectedTargetLabel !== `Y [${ipmaConstructIds.y}]`
    || evidence.checks.ipmaCalculationDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.ipmaCalculationDialog.resultData !== "Standardized (fixed)"
    || evidence.checks.ipmaCalculationDialog.missingData !== "Listwise deletion"
    || evidence.checks.ipmaCalculationDialog.reportedConstructs !== "Direct and indirect structural predecessors only; the target and unrelated constructs are omitted"
    || !evidence.checks.ipmaCalculationDialog.performanceScope.includes("observed-range scaling of standardized composite scores; no theoretical-range correction")
    || evidence.checks.ipmaCalculationDialog.maximumIterations !== "3000"
    || evidence.checks.ipmaCalculationDialog.tolerance !== "1e-7"
    || evidence.checks.ipmaCalculationDialog.unsupportedControls !== 0
    || !evidence.checks.ipmaCalculationDialog.startEnabled
    || evidence.checks.ipmaCalculationDialog.blockers.length !== 0
    || !evidence.checks.ipmaCalculationDialog.predecessorOnly
    || !evidence.checks.ipmaCalculationDialog.observedRange
    || !evidence.checks.ipmaCalculationDialog.noCipmaOrInferenceClaim) {
    throw new Error(`The packaged IPMA setup did not match its exact single-target bounded ready-state contract: ${JSON.stringify(evidence.checks.ipmaCalculationDialog)}`);
  }
  await capture(ipmaCaptureName(79, "dialog"));

  const ipmaActive = ipmaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const ipmaActiveWait = ipmaActive.waitFor({ state: "visible", timeout: 20_000 });
  await startIpma.click();
  await ipmaActiveWait;
  evidence.checks.ipmaRunning = await ipmaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  if (!["queued", "validating", "running"].includes(evidence.checks.ipmaRunning.status)
    || !evidence.checks.ipmaRunning.phase
    || !evidence.checks.ipmaRunning.cancelVisible) {
    throw new Error(`The IPMA calculation did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.ipmaRunning)}`);
  }
  await capture(ipmaCaptureName(80, "running"));

  await waitForSurface("results", 120_000);
  const selectedIpmaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /Importance-Performance Map Analysis/i });
  await selectedIpmaRunOption.waitFor({ state: "attached", timeout: 120_000 });
  const ipmaRunId = await page.locator(".nd-run-select select").inputValue();
  const ipmaNavigation = await inspectIpmaResultTree();
  const ipmaRunDetails = await inspectCurrentRunDetails();
  const ipmaResultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  evidence.checks.ipmaResult = {
    runId: ipmaRunId,
    runLabel: compactVisibleText(await selectedIpmaRunOption.textContent()),
    autoOpenedSurface: "results",
    autoOpenedTable: ipmaNavigation.initialSelectedTable,
    navigation: ipmaNavigation,
    runDetails: ipmaRunDetails,
    resultProperties: ipmaResultProperties,
  };
  if (ipmaRunDetails.properties["Method version"] !== ipmaProvenanceMethodVersion
    || ipmaRunDetails.properties.Method !== "Importance-Performance Map Analysis"
    || ipmaRunDetails.properties.Weighting !== "path"
    || ipmaRunDetails.properties.Preprocessing !== "standardized"
    || Object.prototype.hasOwnProperty.call(ipmaRunDetails.properties, "Recorded seed")
    || Object.prototype.hasOwnProperty.call(ipmaResultProperties, "Recorded seed")
    || ipmaRunDetails.logEntries < 1) {
    throw new Error(`The completed IPMA run details did not expose exact provenance and genuine stored logs: ${JSON.stringify(evidence.checks.ipmaResult)}`);
  }
  await capture(ipmaCaptureName(81, "results"));

  const ipmaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await ipmaExportCommand.click();
  const ipmaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await ipmaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const ipmaXlsxExport = ipmaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await ipmaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const ipmaExportLabels = (await ipmaExportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText);
  evidence.checks.ipmaExport = {
    selectedRunId: ipmaRunId,
    formats: ipmaExportLabels,
    xlsxEnabled: await ipmaXlsxExport.isEnabled(),
    ipmaTablesIncluded: null,
    nativeXlsx: null,
  };
  if (!["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"].every((label) => ipmaExportLabels.includes(label))
    || !evidence.checks.ipmaExport.xlsxEnabled) {
    throw new Error(`The completed IPMA result did not expose the expected enabled native exports: ${JSON.stringify(evidence.checks.ipmaExport)}`);
  }
  if (requestedIpmaNativeExportPath) {
    const ipmaExportTargetPath = await validateRequestedNativeExportPath(requestedIpmaNativeExportPath, "QUICKPLS_IPMA_NATIVE_EXPORT_PATH");
    const ipmaNativeSaveHelper = startWindowsNativeSaveExportHelper({
      targetPath: ipmaExportTargetPath,
      windowTitle: evidence.checks.runtime.title,
      expectedSheets: ["Construct importance and perfor", "Indicator performance", "Calculation scope", "Run provenance"],
      expectedSharedStrings: ["Construct importance and performance", "Indicator performance", "Calculation scope", "Predecessor construct", "Not applied", "Run provenance"],
    });
    let ipmaHelperCompleted = false;
    try {
      const ready = await ipmaNativeSaveHelper.ready;
      if (!ready.passed || ready.event !== "ready") {
        throw new Error(`Native IPMA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
      }
      await ipmaXlsxExport.click();
      const completion = await ipmaNativeSaveHelper.completed;
      ipmaHelperCompleted = true;
      if (!completion.passed) throw new Error(`Native IPMA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
      const expectedFeedback = `Saved ${path.basename(ipmaExportTargetPath)}.`;
      const feedback = ipmaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
      await feedback.waitFor({ state: "visible", timeout: 15_000 });
      const file = await fs.stat(ipmaExportTargetPath);
      const workbookSheets = await inspectXlsxWorkbookSheets(ipmaExportTargetPath);
      evidence.checks.ipmaExport.nativeXlsx = {
        attempted: true,
        targetPath: ipmaExportTargetPath,
        helper: { ready, completion },
        appFeedback: compactVisibleText(await feedback.textContent()),
        file: { size: file.size, isFile: file.isFile() },
        workbookSheets,
      };
      evidence.checks.ipmaExport.ipmaTablesIncluded = ["Construct importance and perfor", "Indicator performance", "Calculation scope"]
        .every((sheet) => workbookSheets.includes(sheet));
      if (!file.isFile() || file.size <= 0
        || evidence.checks.ipmaExport.nativeXlsx.appFeedback !== expectedFeedback
        || !evidence.checks.ipmaExport.ipmaTablesIncluded) {
        throw new Error(`The packaged IPMA XLSX did not contain all method-specific tables with confirmed app feedback: ${JSON.stringify(evidence.checks.ipmaExport)}`);
      }
    } finally {
      if (!ipmaHelperCompleted) ipmaNativeSaveHelper.stop();
    }
  } else {
    evidence.checks.ipmaExport.nativeXlsx = {
      attempted: false,
      reason: "QUICKPLS_IPMA_NATIVE_EXPORT_PATH was not set; the optional real native XLSX Save-dialog and workbook-content gate was not requested.",
    };
  }
  await capture(ipmaCaptureName(82, "export"));
  await ipmaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedIpmaArchive = await inspectSavedIpmaArchive(ipmaProjectPath, ipmaRunId, ipmaConstructIds);
  await reloadToLauncher();
  await openRecentProject(ipmaProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedIpmaOption = page.locator(".nd-run-select select option").filter({ hasText: /Importance-Performance Map Analysis/i }).first();
  await reopenedIpmaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedIpmaRunId = await reopenedIpmaOption.getAttribute("value");
  if (!reopenedIpmaRunId) throw new Error("The reopened IPMA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedIpmaRunId);
  const reopenedIpmaNavigation = await inspectIpmaResultTree();
  evidence.checks.ipmaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    expectedRunId: ipmaRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedIpmaRunId === ipmaRunId
      && await page.locator(".nd-run-select select").inputValue() === ipmaRunId,
    archive: savedIpmaArchive,
    navigation: reopenedIpmaNavigation,
  };
  if (!evidence.checks.ipmaSaveReopen.sameRunRestored
    || reopenedIpmaNavigation.initialSelectedTable !== "Construct importance and performance"
    || !reopenedIpmaNavigation.predecessorOnly
    || !reopenedIpmaNavigation.noPlaceholderOrUnsupportedClaims) {
    throw new Error(`The exact IPMA run and predecessor-only Results did not survive explicit save and reopen: ${JSON.stringify(evidence.checks.ipmaSaveReopen)}`);
  }
  await capture(ipmaCaptureName(83, "reopened"));
  }
  }

  if (!mgaOnly) {
  await seedRecentProject({
    name: ncaProjectName,
    path: ncaProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const ncaRecentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: ncaProjectName });
  await ncaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await ncaRecentRow.count() !== 1 || !(await ncaRecentRow.textContent())?.includes(ncaProjectPath)) {
    throw new Error("The deterministic model-free NCA project was not exposed through one truthful Recent Projects row.");
  }
  await openRecentProject(ncaProjectName);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ncaDatasetStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const ncaDataHeaders = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);

  const ncaTree = await openWorkspaceExplorer(ncaProjectName);
  await ncaTree.waitFor({ state: "visible", timeout: 15_000 });
  await workspaceTreeItem("project", ncaProjectName).click();
  const ncaProjectProperties = await page.locator(".nd-explorer-properties").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await workspaceTreeItem("models", "Models").click();
  const ncaModelProperties = await page.locator(".nd-explorer-properties").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const ncaEditableModelTreeItems = await workspaceTreeItem("model").count();
  const ncaDataTreeItem = workspaceTreeItem("data").filter({ hasText: /nca_native_reference\.csv/i });
  if (await ncaDataTreeItem.count() !== 1) throw new Error("The NCA Project Explorer did not expose exactly one tracked data item.");
  await ncaDataTreeItem.click();
  await page.locator(".nd-explorer-detail-actions").getByRole("button", { name: "Open", exact: true }).click();
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ncaDataToolbarLabels = (await page.locator('.nd-commandbar[role="toolbar"] button').allTextContents()).map(compactVisibleText);
  evidence.checks.ncaFixtureProvisioning.visibleDataset = {
    cases: ncaDatasetStatus.includes("4 cases") ? 4 : null,
    columns: ncaDataHeaders,
    status: ncaDatasetStatus,
    surface: await page.locator(".nd-app").getAttribute("data-surface"),
    projectProperties: ncaProjectProperties,
    modelProperties: ncaModelProperties,
    editableModelTreeItems: ncaEditableModelTreeItems,
    toolbar: ncaDataToolbarLabels,
  };
  if (evidence.checks.ncaFixtureProvisioning.visibleDataset.cases !== 4
    || JSON.stringify(ncaDataHeaders) !== JSON.stringify(["#", "x", "y"])
    || ncaProjectProperties.Models !== "0" || ncaModelProperties.Models !== "0"
    || ncaModelProperties["Active model"] !== "None" || ncaEditableModelTreeItems !== 0
    || !ncaDataToolbarLabels.some((label) => /^Analyze/i.test(label))
    || ncaDataToolbarLabels.some((label) => /^Calculate/i.test(label))) {
    throw new Error(`The visible NCA project was not a four-case, two-variable, zero-model Data workspace with one Analyze command: ${JSON.stringify(evidence.checks.ncaFixtureProvisioning.visibleDataset)}`);
  }
  await capture(ncaCaptureName(84, "fixture-data-no-model"));

  const ncaDialog = await openAnalysisFromDataToolbar();
  const ncaListbox = ncaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const ncaMethods = (await ncaListbox.getByRole("option").locator("strong").allTextContents()).map(compactVisibleText);
  const ncaInitiallySelected = compactVisibleText(await ncaListbox.getByRole("option", { selected: true }).locator("strong").textContent());
  const ncaOption = ncaListbox.getByRole("option", { name: /Necessary Condition Analysis/i });
  if (ncaInitiallySelected !== "Necessary Condition Analysis") await ncaOption.click();
  const ncaX = ncaDialog.locator("#nd-calculation-nca-x");
  const ncaY = ncaDialog.locator("#nd-calculation-nca-y");
  const ncaXOptionsBefore = await ncaX.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    disabled: option.disabled,
  })));
  await ncaX.selectOption("x");
  const ncaYOptionsAfterX = await ncaY.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    disabled: option.disabled,
  })));
  await ncaY.selectOption("y");
  const ncaCeiling = ncaDialog.locator("#nd-calculation-nca-ceiling");
  const ncaCeilingModes = await ncaCeiling.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
  })));
  for (const mode of ["ce_fdh", "cr_fdh", "both"]) await ncaCeiling.selectOption(mode);
  const ncaPermutations = ncaDialog.locator("#nd-calculation-nca-permutations");
  await ncaPermutations.fill(String(ncaPermutationSamples));
  const ncaSeedInput = ncaDialog.locator("#nd-calculation-seed");
  await ncaSeedInput.fill(String(ncaSeed));
  const ncaNoteValue = async (label) => compactVisibleText(await ncaDialog.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const ncaSelectedPanelText = compactVisibleText(await ncaDialog.locator("#nd-calculation-panel").textContent());
  const startNca = ncaDialog.getByRole("button", { name: "Start necessary condition analysis", exact: true });
  evidence.checks.ncaCalculationDialog = {
    methods: ncaMethods,
    initiallySelectedMethod: ncaInitiallySelected,
    selectedMethod: compactVisibleText(await ncaListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await ncaListbox.locator("#nd-calculation-category-standalone").textContent()),
    xOptions: ncaXOptionsBefore,
    yOptionsAfterX: ncaYOptionsAfterX,
    selectedX: await ncaX.inputValue(),
    selectedY: await ncaY.inputValue(),
    ceilingModes: ncaCeilingModes,
    selectedCeiling: await ncaCeiling.inputValue(),
    permutations: {
      value: await ncaPermutations.inputValue(),
      min: await ncaPermutations.getAttribute("min"),
      max: await ncaPermutations.getAttribute("max"),
      step: await ncaPermutations.getAttribute("step"),
    },
    seed: {
      value: await ncaSeedInput.inputValue(),
      min: await ncaSeedInput.getAttribute("min"),
      max: await ncaSeedInput.getAttribute("max"),
    },
    variableData: await ncaNoteValue("Variable data"),
    validatedScope: await ncaNoteValue("Validated scope"),
    unsupportedControls: await ncaDialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
      "#nd-calculation-group-column",
      "#nd-calculation-ipma-target",
    ].join(", ")).count(),
    startEnabled: await startNca.isEnabled(),
    blockers: await ncaDialog.locator(".nd-blocker li").allTextContents(),
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(compactVisibleText(await ncaDialog.locator(".nd-blocker").textContent().catch(() => ""))),
    noBroaderProductClaim: !/SmartPLS|full NCA|IPMA integration/i.test(ncaSelectedPanelText),
    noIdleRunState: await ncaDialog.locator(".nd-run-progress, progress, .nd-run-details").count() === 0,
  };
  const expectedNcaXOptions = [
    { value: "", label: "Select a numeric variable", disabled: false },
    { value: "x", label: "x", disabled: false },
    { value: "y", label: "y", disabled: false },
  ];
  const expectedNcaYOptions = [
    { value: "", label: "Select a different numeric variable", disabled: false },
    { value: "x", label: "x", disabled: true },
    { value: "y", label: "y", disabled: false },
  ];
  const expectedNcaCeilingModes = [
    { value: "both", label: "CE-FDH and CR-FDH" },
    { value: "ce_fdh", label: "CE-FDH" },
    { value: "cr_fdh", label: "CR-FDH" },
  ];
  if (JSON.stringify(ncaMethods) !== JSON.stringify(expectedOptionLabels)
    || evidence.checks.ncaCalculationDialog.initiallySelectedMethod !== "Necessary Condition Analysis"
    || evidence.checks.ncaCalculationDialog.selectedMethod !== "Necessary Condition Analysis"
    || evidence.checks.ncaCalculationDialog.category !== "Standalone analysis"
    || JSON.stringify(ncaXOptionsBefore) !== JSON.stringify(expectedNcaXOptions)
    || JSON.stringify(ncaYOptionsAfterX) !== JSON.stringify(expectedNcaYOptions)
    || evidence.checks.ncaCalculationDialog.selectedX !== "x" || evidence.checks.ncaCalculationDialog.selectedY !== "y"
    || JSON.stringify(ncaCeilingModes) !== JSON.stringify(expectedNcaCeilingModes)
    || evidence.checks.ncaCalculationDialog.selectedCeiling !== "both"
    || JSON.stringify(evidence.checks.ncaCalculationDialog.permutations) !== JSON.stringify({ value: "19", min: "1", max: "10000", step: "1" })
    || JSON.stringify(evidence.checks.ncaCalculationDialog.seed) !== JSON.stringify({ value: "20260811", min: "0", max: "4294967295" })
    || evidence.checks.ncaCalculationDialog.variableData !== "Observed numeric values (fixed)"
    || evidence.checks.ncaCalculationDialog.validatedScope !== "Numeric observed-variable CE-FDH and CR-FDH analysis with observed-range bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and broader ceiling variants are not included."
    || evidence.checks.ncaCalculationDialog.unsupportedControls !== 0
    || !evidence.checks.ncaCalculationDialog.startEnabled || evidence.checks.ncaCalculationDialog.blockers.length !== 0
    || !evidence.checks.ncaCalculationDialog.noModelBlocker || !evidence.checks.ncaCalculationDialog.noBroaderProductClaim
    || !evidence.checks.ncaCalculationDialog.noIdleRunState) {
    throw new Error(`The packaged NCA setup did not match its exact model-free numeric X/Y, CE-FDH/CR-FDH, and seeded permutation contract: ${JSON.stringify(evidence.checks.ncaCalculationDialog)}`);
  }
  await capture(ncaCaptureName(85, "dialog"));

  const ncaActive = ncaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const ncaActiveWait = ncaActive.waitFor({ state: "visible", timeout: 20_000 });
  await startNca.click();
  await ncaActiveWait;
  evidence.checks.ncaRunning = await ncaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  const ncaProgressValue = Number(evidence.checks.ncaRunning.progressValue);
  const ncaProgressMax = Number(evidence.checks.ncaRunning.progressMax);
  if (!["queued", "validating", "running"].includes(evidence.checks.ncaRunning.status)
    || !evidence.checks.ncaRunning.phase || !evidence.checks.ncaRunning.message
    || evidence.checks.ncaRunning.logEntries < 1 || !evidence.checks.ncaRunning.cancelVisible
    || !Number.isFinite(ncaProgressValue) || !Number.isFinite(ncaProgressMax)
    || ncaProgressMax <= 0 || ncaProgressValue < 0 || ncaProgressValue > ncaProgressMax) {
    throw new Error(`The NCA calculation did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.ncaRunning)}`);
  }
  await capture(ncaCaptureName(86, "running"));

  await waitForSurface("results", 120_000);
  const selectedNcaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /Necessary Condition Analysis/i });
  await selectedNcaRunOption.waitFor({ state: "attached", timeout: 120_000 });
  const ncaRunId = await page.locator(".nd-run-select select").inputValue();
  const ncaNavigation = await inspectNcaResultTree();
  const ncaRunDetails = await inspectCurrentRunDetails();
  const ncaResultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const visibleEditModel = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Edit Model$/i });
  const visibleEditData = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Edit Data$/i });
  evidence.checks.ncaResult = {
    runId: ncaRunId,
    runLabel: compactVisibleText(await selectedNcaRunOption.textContent()),
    autoOpenedSurface: "results",
    autoOpenedTable: ncaNavigation.initialSelectedTable,
    navigation: ncaNavigation,
    runDetails: ncaRunDetails,
    resultProperties: ncaResultProperties,
    editModelCommand: {
      count: await visibleEditModel.count(),
      enabled: await visibleEditModel.count() === 1 ? await visibleEditModel.isEnabled() : false,
    },
    editDataCommand: {
      count: await visibleEditData.count(),
      enabled: await visibleEditData.count() === 1 ? await visibleEditData.isEnabled() : false,
    },
  };
  if (ncaRunDetails.properties["Method version"] !== ncaMethodVersion
    || ncaRunDetails.properties.Method !== "Necessary Condition Analysis"
    || ncaRunDetails.properties["Recorded seed"] !== String(ncaSeed)
    || ncaRunDetails.properties["Condition (X)"] !== "x" || ncaRunDetails.properties["Outcome (Y)"] !== "y"
    || ncaRunDetails.properties.Observations !== "4" || ncaRunDetails.properties["Ceiling lines"] !== "CE-FDH and CR-FDH"
    || ncaRunDetails.properties["Requested permutations"] !== String(ncaPermutationSamples)
    || ncaRunDetails.properties["Usable permutations"] !== String(ncaPermutationSamples)
    || ncaRunDetails.properties["Missing data"] !== "Listwise deletion"
    || "Weighting" in ncaRunDetails.properties || "Preprocessing" in ncaRunDetails.properties
    || ncaRunDetails.logEntries < 1
    || ncaResultProperties.Method !== "Necessary Condition Analysis"
    || ncaResultProperties.Status !== "Completed"
    || ncaResultProperties["Condition (X)"] !== "x" || ncaResultProperties["Outcome (Y)"] !== "y"
    || ncaResultProperties.Observations !== "4" || ncaResultProperties["Ceiling lines"] !== "CE-FDH and CR-FDH"
    || ncaResultProperties["Requested permutations"] !== String(ncaPermutationSamples)
    || ncaResultProperties["Usable permutations"] !== String(ncaPermutationSamples)
    || ncaResultProperties["Recorded seed"] !== String(ncaSeed)
    || evidence.checks.ncaResult.editModelCommand.count !== 0
    || evidence.checks.ncaResult.editDataCommand.count !== 1 || !evidence.checks.ncaResult.editDataCommand.enabled) {
    throw new Error(`The completed NCA Results did not expose exact provenance, properties, stored logs, and a model-free return boundary: ${JSON.stringify(evidence.checks.ncaResult)}`);
  }
  await capture(ncaCaptureName(87, "results"));

  const ncaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await ncaExportCommand.click();
  const ncaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await ncaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const ncaXlsxExport = ncaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await ncaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const ncaExportLabels = (await ncaExportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText);
  evidence.checks.ncaExport = {
    selectedRunId: ncaRunId,
    formats: ncaExportLabels,
    xlsxEnabled: await ncaXlsxExport.isEnabled(),
    modelDiagramCount: await ncaExportDialog.getByRole("button", { name: /Model diagram/i }).count(),
    ncaTablesIncluded: null,
    nativeXlsx: null,
  };
  const expectedNcaExportLabels = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Print / PDF"];
  if (JSON.stringify(ncaExportLabels) !== JSON.stringify(expectedNcaExportLabels)
    || !evidence.checks.ncaExport.xlsxEnabled || evidence.checks.ncaExport.modelDiagramCount !== 0) {
    throw new Error(`The completed model-free NCA result did not expose the exact table/provenance exports: ${JSON.stringify(evidence.checks.ncaExport)}`);
  }
  if (!requestedNcaNativeExportPath) {
    throw new Error("QUICKPLS_NCA_NATIVE_EXPORT_PATH is required for authoritative packaged NCA export acceptance; the harness will not replace genuine XLSX Save-dialog and workbook-content evidence with enabled-button assertions.");
  }
  const ncaExportTargetPath = await validateRequestedNativeExportPath(requestedNcaNativeExportPath, "QUICKPLS_NCA_NATIVE_EXPORT_PATH");
  const ncaNativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: ncaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: ["Ceiling effect sizes and permut", "CR-FDH ceiling coefficients", "Observed-range bottlenecks", "Calculation scope", "Run provenance"],
    expectedSharedStrings: [
      "Ceiling effect sizes and permutation inference", "CR-FDH ceiling coefficients", "Observed-range bottlenecks",
      "Calculation scope", "Run provenance", "CE-FDH", "CR-FDH", "Condition variable (X)",
      "Necessary Condition Analysis", "nca_v2", String(ncaSeed),
    ],
  });
  let ncaHelperCompleted = false;
  try {
    const ready = await ncaNativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native NCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await ncaXlsxExport.click();
    const completion = await ncaNativeSaveHelper.completed;
    ncaHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native NCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(ncaExportTargetPath)}.`;
    const feedback = ncaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(ncaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(ncaExportTargetPath);
    evidence.checks.ncaExport.nativeXlsx = {
      attempted: true,
      targetPath: ncaExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    evidence.checks.ncaExport.ncaTablesIncluded = [
      "Ceiling effect sizes and permut", "CR-FDH ceiling coefficients", "Observed-range bottlenecks", "Calculation scope", "Run provenance",
    ].every((sheet) => workbookSheets.includes(sheet));
    if (!file.isFile() || file.size <= 0 || evidence.checks.ncaExport.nativeXlsx.appFeedback !== expectedFeedback
      || !evidence.checks.ncaExport.ncaTablesIncluded) {
      throw new Error(`The packaged NCA XLSX did not contain all exact NCA and provenance sheets: ${JSON.stringify(evidence.checks.ncaExport)}`);
    }
  } finally {
    if (!ncaHelperCompleted) ncaNativeSaveHelper.stop();
  }
  await capture(ncaCaptureName(88, "export"));
  await ncaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedNcaArchive = await inspectSavedNcaArchive(ncaProjectPath, ncaRunId);
  await reloadToLauncher();
  await openRecentProject(ncaProjectName);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedNcaOption = page.locator(".nd-run-select select option").filter({ hasText: /Necessary Condition Analysis/i }).first();
  await reopenedNcaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedNcaRunId = await reopenedNcaOption.getAttribute("value");
  if (!reopenedNcaRunId) throw new Error("The reopened NCA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedNcaRunId);
  const reopenedNcaNavigation = await inspectNcaResultTree();
  evidence.checks.ncaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    expectedRunId: ncaRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedNcaRunId === ncaRunId
      && await page.locator(".nd-run-select select").inputValue() === ncaRunId,
    sameVisibleEffectsRestored: JSON.stringify(reopenedNcaNavigation.effects.rows) === JSON.stringify(ncaNavigation.effects.rows),
    archive: savedNcaArchive,
    navigation: reopenedNcaNavigation,
  };
  if (!evidence.checks.ncaSaveReopen.sameRunRestored
    || !evidence.checks.ncaSaveReopen.sameVisibleEffectsRestored
    || reopenedNcaNavigation.initialSelectedTable !== "Ceiling effect sizes and permutation inference"
    || !reopenedNcaNavigation.bottlenecksMatch || !reopenedNcaNavigation.noModelOrQualityTree
    || !reopenedNcaNavigation.noPlaceholder || !reopenedNcaNavigation.noBroaderNcaClaim) {
    throw new Error(`The exact model-free NCA run and nca_v2 Results did not survive explicit save and reopen: ${JSON.stringify(evidence.checks.ncaSaveReopen)}`);
  }
  await capture(ncaCaptureName(89, "reopened"));
  }
  }

  if (evidence.consoleErrors.length) throw new Error(`Console errors: ${JSON.stringify(evidence.consoleErrors)}`);
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.message : String(error));
  await capture("99-tauri-native-failure-state-1440x900.png").catch(() => undefined);
  process.exitCode = 1;
} finally {
  if (recentProjectsSeeded) {
    evidence.checks.recentProjectsRestored = await page.evaluate(({ key, prior }) => {
      if (prior === null) window.localStorage.removeItem(key);
      else window.localStorage.setItem(key, prior);
      return window.localStorage.getItem(key) === prior;
    }, { key: recentProjectsKey, prior: priorRecentProjectsRaw }).catch(() => false);
  }
  if (evidence.focusedRun) evidence.focusedRun.completedAt = new Date().toISOString();
  evidence.passed = evidence.failures.length === 0 && evidence.consoleErrors.length === 0;
  await writeAcceptanceEvidence();
}

console.log(JSON.stringify({
  passed: evidence.passed,
  checks: Object.keys(evidence.checks),
  screenshots: evidence.screenshots.length,
  consoleErrors: evidence.consoleErrors.length,
  failures: evidence.failures,
  reportPath,
  scopedReportPath,
}, null, 2));
process.exit(evidence.failures.length ? 1 : 0);
