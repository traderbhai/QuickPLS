import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const SCREENS = path.join(RESULTS, "screens", "v11");
const PORT = 5211;
const PREVIEW_URL = `http://127.0.0.1:${PORT}/`;
const OUTPUT = path.join(RESULTS, "v11_desktop_ux_smoke.json");

const screenshots = {
  data: path.join(SCREENS, "v11_01_data_1440x900.png"),
  model: path.join(SCREENS, "v11_02_model_1440x900.png"),
  validate: path.join(SCREENS, "v11_03_validate_1440x900.png"),
  run: path.join(SCREENS, "v11_04_run_1440x900.png"),
  results: path.join(SCREENS, "v11_05_results_1440x900.png"),
  report: path.join(SCREENS, "v11_06_report_1440x900.png"),
  completedResults: path.join(SCREENS, "v11_07_results_completed_1440x900.png"),
  completedModelOverlay: path.join(SCREENS, "v11_08_model_completed_overlay_1440x900.png"),
  completedReport: path.join(SCREENS, "v11_09_report_completed_1440x900.png"),
};

await mkdir(SCREENS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], {
  cwd: ROOT,
  windowsHide: true,
  stdio: ["ignore", "pipe", "pipe"],
});

let logs = "";
server.stdout.on("data", (data) => { logs += data.toString(); });
server.stderr.on("data", (data) => { logs += data.toString(); });

async function waitForUrl(url, deadlineMs = 45_000) {
  const start = Date.now();
  while (Date.now() - start < deadlineMs) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 2_000);
      const response = await fetch(url, { signal: controller.signal });
      clearTimeout(timeout);
      if (response.ok) return;
    } catch {
      // Keep polling until the preview server is ready.
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Timed out waiting for Vite preview. Logs: ${logs.slice(-2000)}`);
}

async function openWorkspace(page, label, screenshot) {
  await page.locator(".nav-item").filter({ hasText: new RegExp(`^${label}$`) }).click();
  await page.waitForTimeout(450);
  await page.screenshot({ path: screenshot, fullPage: true });
}

let browser;
try {
  await waitForUrl(PREVIEW_URL);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });

  await page.goto(`${PREVIEW_URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await openWorkspace(page, "Data", screenshots.data);
  const dataGrid = page.locator(".data-grid").first();
  await dataGrid.focus();
  const dataMetrics = await dataGrid.evaluate((element) => ({
    focusedDataGrid: document.activeElement === element,
    dataGridRole: element.getAttribute("role"),
    dataGridTabIndex: element.getAttribute("tabindex"),
    dataGridLabel: element.getAttribute("aria-label"),
    dataCaption: element.querySelector("caption")?.textContent ?? null,
  }));

  await openWorkspace(page, "Model", screenshots.model);
  const modelMetrics = await page.evaluate(() => {
    const overlayText = document.querySelector(".canvas-overlay-status")?.textContent ?? "";
    const mojibake = ["R\u00c3", "R\u00c2", "R\ufffd"];
    return {
      academicLatents: document.querySelectorAll(".smartpls-latent-node").length,
      academicIndicators: document.querySelectorAll(".smartpls-indicator-node").length,
      toolbar: Boolean(document.querySelector(".canvas-toolbar")),
      overlayStatus: overlayText,
      overlayStatusMojibake: mojibake.some((value) => overlayText.includes(value)),
      nextAction: document.querySelector(".canvas-next-action")?.textContent ?? "",
      shortcutHelp: document.body.textContent?.includes("Shortcuts: P path, C covariance, V select, F fit view") ?? false,
    };
  });

  await openWorkspace(page, "Validate", screenshots.validate);
  const validateMetrics = await page.evaluate(() => ({
    readinessItems: document.querySelectorAll(".readiness-item").length,
    methodRows: document.querySelectorAll(".method-row").length,
    statusBadges: document.querySelectorAll(".status-text").length,
    hasReadinessAction: [...document.querySelectorAll(".readiness-action")].some((button) => button.textContent?.includes("Open")),
  }));

  await openWorkspace(page, "Run", screenshots.run);
  const runMetrics = await page.evaluate(() => ({
    readinessPanel: Boolean(document.querySelector(".readiness-panel")),
    runDisabledReason: Boolean(document.querySelector(".disabled-reason")),
    runLaunchCard: Boolean(document.querySelector(".run-launch-card")),
    runBlockerIsActionable: (document.body.textContent?.includes("desktop runtime") || document.body.textContent?.includes("reproducible fingerprint")) ?? false,
    hasReadinessAction: [...document.querySelectorAll(".readiness-action")].some((button) => button.textContent?.includes("Open")),
  }));

  await openWorkspace(page, "Results", screenshots.results);
  const resultsMetrics = await page.evaluate(() => ({
    emptyResults: document.body.textContent?.includes("No completed results") ?? false,
    readinessPanel: Boolean(document.querySelector(".readiness-panel")),
  }));

  await openWorkspace(page, "Report", screenshots.report);
  const reportMetrics = await page.evaluate(() => {
    const bodyText = document.body.textContent ?? "";
    const mojibake = ["R\u00c3", "R\u00c2", "R\ufffd"];
    return {
      publicationPreview: Boolean(document.querySelector(".publication-preview-shell .diagram-preview svg")),
      exportReason: Boolean(document.querySelector(".top-export-reason")),
      rSquaredMojibake: mojibake.some((value) => bodyText.includes(value)),
      rSquaredStructuralLabel: Boolean([...document.querySelectorAll("label")].find((label) => label.textContent === "R2" && label.querySelector("sup")?.textContent === "2")),
    };
  });

  const workflowMetrics = await page.evaluate(() => ({
    workflowLabels: [...document.querySelectorAll(".workflow-step")].map((element) => element.textContent?.trim()),
    navLabels: [...document.querySelectorAll(".nav-item")].map((element) => element.textContent?.trim()),
    footerReadinessPills: document.querySelectorAll(".status-readiness-pill").length,
    footerReadinessLabels: [...document.querySelectorAll(".status-readiness-pill")].map((element) => element.textContent?.trim()),
    footerReadinessNamed: document.querySelector(".status-readiness-strip")?.getAttribute("aria-label") ?? "",
  }));

  const smokeHarnessMetrics = await page.evaluate(() => {
    window.__QUICKPLS_SMOKE__?.addCompletedRun();
    return { available: Boolean(window.__QUICKPLS_SMOKE__) };
  });
  await page.waitForTimeout(450);
  await page.screenshot({ path: screenshots.completedResults, fullPage: true });
  const completedResultsMetrics = await page.evaluate(() => ({
    hasCompletedRun: document.body.textContent?.includes("PLS path modeling core run") ?? false,
    hasResultSummary: Boolean(document.querySelector(".result-summary")),
    hasQualityWarning: document.body.textContent?.includes("Validated for the documented QuickPLS v1.0.0 supported scope") ?? false,
    pathRows: [...document.querySelectorAll(".result-summary table tbody tr")].filter((row) => row.textContent?.includes("->")).length,
    rSquaredValues: [...document.querySelectorAll(".result-summary small")].filter((item) => item.textContent?.startsWith("R2")).length,
  }));

  await openWorkspace(page, "Model", screenshots.completedModelOverlay);
  const completedModelMetrics = await page.evaluate(() => ({
    overlayHasLoadings: ["0.842", "0.874", "0.902", "0.913"].every((value) => document.body.textContent?.includes(value)),
    overlayHasPaths: ["0.403", "0.327", "0.544"].every((value) => document.body.textContent?.includes(value)),
    overlayHasRSquared: document.body.textContent?.includes("R\u00b2 0.544") || document.body.textContent?.includes("R2 0.544"),
    overlayStatusActive: document.querySelector(".canvas-overlay-status")?.textContent?.includes("Result overlay active") ?? false,
    hasEditChrome: Boolean(document.querySelector(".canvas-toolbar")),
  }));

  await openWorkspace(page, "Report", screenshots.completedReport);
  const completedReportMetrics = await page.evaluate(() => {
    const svgText = document.querySelector(".publication-preview-shell .diagram-preview svg")?.textContent ?? "";
    return {
      runSelected: document.body.textContent?.includes("run selected") ?? false,
      wysiwygStatement: document.body.textContent?.includes("WYSIWYG SVG export with selected run overlays") ?? false,
      svgHasLoadings: ["0.842", "0.874", "0.902", "0.913"].every((value) => svgText.includes(value)),
      svgHasPaths: ["0.403", "0.327", "0.544"].every((value) => svgText.includes(value)),
      svgHasRSquared: svgText.includes("R\u00b2 0.544") || svgText.includes("R2 0.544"),
      exportReasonExplainsRuntime: document.querySelector(".top-export-reason")?.textContent?.includes("XLSX export requires the desktop runtime") ?? false,
    };
  });

  const metrics = { dataMetrics, modelMetrics, validateMetrics, runMetrics, resultsMetrics, reportMetrics, workflowMetrics, smokeHarnessMetrics, completedResultsMetrics, completedModelMetrics, completedReportMetrics };
  const passed = errors.length === 0
    && dataMetrics.focusedDataGrid
    && dataMetrics.dataGridRole === "region"
    && dataMetrics.dataGridTabIndex === "0"
    && modelMetrics.academicLatents >= 4
    && modelMetrics.academicIndicators >= 9
    && modelMetrics.toolbar
    && modelMetrics.overlayStatus.includes("Model-only diagram")
    && !modelMetrics.overlayStatusMojibake
    && modelMetrics.nextAction.includes("Open")
    && validateMetrics.readinessItems >= 5
    && validateMetrics.hasReadinessAction
    && runMetrics.readinessPanel
    && runMetrics.runDisabledReason
    && runMetrics.runBlockerIsActionable
    && runMetrics.hasReadinessAction
    && resultsMetrics.emptyResults
    && reportMetrics.publicationPreview
    && reportMetrics.exportReason
    && reportMetrics.rSquaredStructuralLabel
    && !reportMetrics.rSquaredMojibake
    && ["Data", "Model", "Validate", "Run", "Results", "Report"].every((label) => workflowMetrics.workflowLabels.includes(label))
    && workflowMetrics.footerReadinessPills >= 6
    && workflowMetrics.footerReadinessNamed.includes("Persistent analysis readiness")
    && smokeHarnessMetrics.available
    && completedResultsMetrics.hasCompletedRun
    && completedResultsMetrics.hasResultSummary
    && completedResultsMetrics.pathRows >= 5
    && completedResultsMetrics.rSquaredValues >= 2
    && completedModelMetrics.overlayHasLoadings
    && completedModelMetrics.overlayHasPaths
    && completedModelMetrics.overlayHasRSquared
    && completedModelMetrics.overlayStatusActive
    && completedReportMetrics.runSelected
    && completedReportMetrics.wysiwygStatement
    && completedReportMetrics.svgHasLoadings
    && completedReportMetrics.svgHasPaths
    && completedReportMetrics.svgHasRSquared
    && completedReportMetrics.exportReasonExplainsRuntime;

  const report = {
    schema_version: 1,
    target: "v1.1 desktop UX workflow smoke",
    viewport: { width: 1440, height: 900 },
    passed,
    metrics,
    errors,
    screenshots,
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
