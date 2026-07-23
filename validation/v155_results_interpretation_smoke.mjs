import { execFileSync, spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v155", "results-interpretation");
const OUTPUT = path.join(RESULTS, "v155_results_interpretation_smoke.json");
const PORT = 53156;
const URL = `http://127.0.0.1:${PORT}/`;

await mkdir(ARTIFACTS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], {
  cwd: ROOT,
  windowsHide: true,
  stdio: ["ignore", "pipe", "pipe"],
});

let logs = "";
server.stdout.on("data", (data) => { logs += data.toString(); });
server.stderr.on("data", (data) => { logs += data.toString(); });

async function waitForUrl() {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(URL, { signal: AbortSignal.timeout(1500) });
      if (response.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Timed out waiting for Vite preview. ${logs.slice(-1200)}`);
}

async function screenshot(page, name) {
  const file = path.join(ARTIFACTS, name);
  await page.screenshot({ path: file, fullPage: true });
  return file;
}

async function selectTab(page, label) {
  await page.evaluate((tabLabel) => {
    const button = Array.from(document.querySelectorAll("button")).find((item) => item.textContent?.trim() === tabLabel);
    if (!button) throw new Error(`Missing result tab: ${tabLabel}`);
    button.click();
  }, label);
  await page.waitForTimeout(180);
}

async function readTab(page, label, screenshotName) {
  await selectTab(page, label);
  await page.evaluate(() => {
    const firstGuidance = Array.from(document.querySelectorAll(".result-section-actions button")).find((item) => item.textContent?.trim() === "Interpretation");
    firstGuidance?.click();
  });
  await page.waitForTimeout(100);
  const file = await screenshot(page, screenshotName);
  return await page.evaluate((shot) => {
    const bodyText = document.body.textContent ?? "";
    return {
      screenshot: shot,
      sections: Array.from(document.querySelectorAll(".result-section-title strong")).map((item) => item.textContent?.trim()).filter(Boolean),
      interpretationPanels: Array.from(document.querySelectorAll(".interpretation-panel summary")).map((item) => item.textContent?.replace(/\s+/g, " ").trim()).filter(Boolean),
      detailRows: document.querySelectorAll(".result-row-detail").length,
      tableActions: Array.from(document.querySelectorAll(".result-section-actions button")).map((item) => item.textContent?.trim()).filter(Boolean),
      comparisonSelector: document.querySelectorAll(".comparison-selector label").length,
      hasMojibake: new RegExp(["\\u00c2", "\\u00c3", "\\ufffd", "R\\u00c2", "Q\\u00c2", "f\\u00c2"].join("|")).test(bodyText),
      hasOldScope: bodyText.includes("QuickPLS v" + "1.0.0 supported scope"),
    };
  }, file);
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });

  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });
  await page.evaluate(() => {
    window.__QUICKPLS_SMOKE__.addCompletedRun();
    window.__QUICKPLS_SMOKE__.addComparisonRun();
    window.__QUICKPLS_SMOKE__.selectEdge("comp-cusa");
    window.__QUICKPLS_SMOKE__.setView("runs");
  });
  await page.waitForSelector(".researcher-result-card", { timeout: 10_000 });

  const tabs = {
    overview: await readTab(page, "Overview", "01_overview.png"),
    measurement: await readTab(page, "Measurement", "02_measurement.png"),
    structural: await readTab(page, "Structural", "03_structural.png"),
    validity: await readTab(page, "Validity", "04_validity.png"),
    inference: await readTab(page, "Inference", "05_inference.png"),
    prediction: await readTab(page, "Prediction", "06_prediction.png"),
    groups: await readTab(page, "Groups", "07_groups.png"),
    diagnostics: await readTab(page, "Diagnostics", "08_diagnostics.png"),
    interpretation: await readTab(page, "Interpretation", "09_interpretation.png"),
    comparison: await readTab(page, "Comparison", "10_comparison.png"),
  };

  await page.setViewportSize({ width: 1280, height: 800 });
  const desktop1280 = await screenshot(page, "11_comparison_1280.png");

  await selectTab(page, "Structural");
  await page.evaluate(() => {
    const row = Array.from(document.querySelectorAll(".result-table-section table tbody tr"))
      .find((item) => !item.classList.contains("result-path-row"));
    if (!row) throw new Error("No result row available for detail smoke");
    row.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  await page.waitForTimeout(150);
  const rowDetail = await screenshot(page, "12_row_detail.png");
  const rowDetailVisible = await page.locator(".result-row-detail").count();

  const checklist = {
    overview_has_path_guidance: tabs.overview.interpretationPanels.some((text) => text.includes("Path coefficients")),
    measurement_has_loading_guidance: tabs.measurement.interpretationPanels.some((text) => text.includes("Outer loadings")),
    structural_has_r2_and_vif_guidance: tabs.structural.sections.includes("R² and adjusted R²") && tabs.structural.interpretationPanels.length > 0,
    validity_has_reliability_and_discriminant_guidance: tabs.validity.interpretationPanels.some((text) => text.includes("Reliability")) || tabs.validity.interpretationPanels.some((text) => text.includes("Discriminant")),
    inference_has_guidance: tabs.inference.interpretationPanels.some((text) => text.includes("Inference")),
    prediction_has_guidance_or_empty: tabs.prediction.interpretationPanels.some((text) => text.includes("Prediction")) || tabs.prediction.sections.includes("Blindfolding Q²"),
    groups_has_clear_state: tabs.groups.interpretationPanels.some((text) => text.includes("Groups")) || tabs.groups.sections.length === 0,
    diagnostics_has_scope_and_extended_guidance: tabs.diagnostics.sections.includes("Warnings and scope status"),
    interpretation_has_report_wording: tabs.interpretation.sections.includes("Copyable report wording"),
    comparison_has_real_tables: tabs.comparison.sections.includes("Path coefficient deltas") && tabs.comparison.sections.includes("R² deltas") && tabs.comparison.comparisonSelector >= 2,
    table_actions_present: Object.values(tabs).some((tab) => tab.tableActions.includes("Copy table") && tab.tableActions.includes("Interpretation")),
    row_detail_visible: rowDetailVisible > 0,
    screenshots_written: Object.values(tabs).every((tab) => Boolean(tab.screenshot)) && Boolean(desktop1280) && Boolean(rowDetail),
    no_stale_scope_or_mojibake: Object.values(tabs).every((tab) => !tab.hasOldScope && !tab.hasMojibake),
    no_console_errors: errors.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v1.5.5 results interpretation polish smoke",
    passed: Object.values(checklist).every(Boolean),
    checklist,
    tabs,
    screenshots: { desktop1280, rowDetail },
    errors,
  };
  await writeFile(OUTPUT, JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  if (!result.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (server.pid) {
    try {
      execFileSync("taskkill.exe", ["/PID", String(server.pid), "/T", "/F"], { stdio: "ignore" });
    } catch {
      server.kill();
    }
  } else {
    server.kill();
  }
}
