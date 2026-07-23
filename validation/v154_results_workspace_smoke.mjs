import { execFileSync, spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v154", "results-workspace");
const OUTPUT = path.join(RESULTS, "v154_results_workspace_smoke.json");
const PORT = 53155;
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
  await page.waitForTimeout(150);
}

async function readTab(page, label, screenshotName) {
  await selectTab(page, label);
  const file = await screenshot(page, screenshotName);
  const state = await page.evaluate(() => {
    const bodyText = document.body.textContent ?? "";
    return {
      sections: Array.from(document.querySelectorAll(".result-section-title strong")).map((item) => item.textContent?.trim()).filter(Boolean),
      metricTiles: Array.from(document.querySelectorAll(".result-metric-tile")).map((item) => item.textContent?.replace(/\s+/g, " ").trim()).filter(Boolean),
      emptyTitle: document.querySelector(".result-section-empty strong")?.textContent?.trim() ?? null,
      tableCount: document.querySelectorAll(".result-table-section").length,
      bootstrapTableCount: document.querySelectorAll(".bootstrap-summary table").length,
      activeResultRows: document.querySelectorAll(".active-result-row").length,
      hasMethodPayloadDump: Boolean(document.querySelector(".result-sections .method-results")),
      hasMojibake: /\u00c2|\u00c3|\ufffd/.test(bodyText),
      hasOldScope: bodyText.includes("QuickPLS v" + "1.0.0 supported scope"),
    };
  });
  return { ...state, screenshot: file };
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
  await page.evaluate(() => window.__QUICKPLS_SMOKE__.addCompletedRun());
  await page.evaluate(() => window.__QUICKPLS_SMOKE__.selectEdge("comp-cusa"));
  await page.waitForSelector(".researcher-result-card", { timeout: 10_000 });

  const tabs = {
    summary: await readTab(page, "Summary", "01_summary.png"),
    measurement: await readTab(page, "Measurement Model", "02_measurement.png"),
    structural: await readTab(page, "Structural Model", "03_structural.png"),
    quality: await readTab(page, "Reliability and Validity", "04_quality.png"),
    inference: await readTab(page, "Inference", "05_inference.png"),
    prediction: await readTab(page, "Prediction", "06_prediction.png"),
    groups: await readTab(page, "Groups", "07_groups.png"),
    diagnostics: await readTab(page, "Diagnostics", "08_diagnostics.png"),
  };

  const checklist = {
    summary_has_kpis_paths_and_effects: tabs.summary.metricTiles.some((text) => text.includes("R²")) && tabs.summary.sections.includes("Path coefficients") && tabs.summary.sections.includes("Total effects"),
    measurement_has_outer_and_cross_loading_sections: tabs.measurement.sections.includes("Outer loadings and weights") && tabs.measurement.sections.includes("Cross-loadings"),
    structural_has_effects_r2_vif_and_f2: tabs.structural.sections.includes("Path coefficients") && tabs.structural.sections.includes("Total effects") && tabs.structural.sections.includes("R² and adjusted R²") && tabs.structural.sections.includes("Inner VIF") && tabs.structural.sections.includes("Cohen f² effect sizes"),
    quality_has_reliability_htmt_and_fornell_larcker: tabs.quality.sections.includes("Construct reliability and convergent validity") && tabs.quality.sections.includes("Fornell-Larcker criterion"),
    inference_has_bootstrap_table: tabs.inference.bootstrapTableCount > 0 && tabs.inference.sections.includes("Mediation effects") === false,
    prediction_has_q2_or_clear_empty_state: tabs.prediction.sections.includes("Blindfolding Q²") || tabs.prediction.emptyTitle === "Prediction outputs not run",
    groups_are_not_generic_dump_without_payloads: tabs.groups.emptyTitle === "No group or segmentation payloads" && !tabs.groups.hasMethodPayloadDump,
    diagnostics_has_provenance_and_scope: tabs.diagnostics.sections.includes("Run provenance") && tabs.diagnostics.sections.includes("Warnings and scope status"),
    diagram_selection_highlights_result_rows: tabs.summary.activeResultRows > 0 && tabs.measurement.activeResultRows > 0 && tabs.structural.activeResultRows > 0,
    no_stale_scope_or_mojibake: Object.values(tabs).every((tab) => !tab.hasOldScope && !tab.hasMojibake),
    screenshots_written: Object.values(tabs).every((tab) => Boolean(tab.screenshot)),
    no_console_errors: errors.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS results workspace deep smoke",
    passed: Object.values(checklist).every(Boolean),
    checklist,
    tabs,
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
    try {
      execFileSync("powershell.exe", ["-NoProfile", "-Command", `(Get-NetTCPConnection -LocalPort ${PORT} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess -Unique) | ForEach-Object { Stop-Process -Id $_ -Force }`], { stdio: "ignore" });
    } catch {
      // Best-effort cleanup for detached Vite children.
    }
  } else {
    server.kill();
  }
}
