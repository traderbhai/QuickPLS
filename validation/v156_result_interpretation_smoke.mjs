import { execFileSync, spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v156", "result-interpretation");
const OUTPUT = path.join(RESULTS, "v156_result_interpretation_smoke.json");
const PORT = 53157;
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
  await page.waitForTimeout(200);
}

async function readTab(page, label, screenshotName) {
  await selectTab(page, label);
  const file = await screenshot(page, screenshotName);
  return await page.evaluate((shot) => {
    const text = document.body.textContent ?? "";
    return {
      screenshot: shot,
      findingCards: document.querySelectorAll(".finding-card").length,
      findingPanels: document.querySelectorAll(".finding-panel").length,
      checklists: Array.from(document.querySelectorAll(".finding-checklist .result-section-title strong")).map((item) => item.textContent?.trim()).filter(Boolean),
      reportWording: text.includes("Report wording"),
      rowDetails: document.querySelectorAll(".result-row-detail").length,
      copyInterpretationButtons: Array.from(document.querySelectorAll("button")).filter((item) => /Copy interpretation|Copy checklist/.test(item.textContent ?? "")).length,
      hasValueSpecificCopy: /coefficient|loading|HTMT|VIF|R2|bootstrap|path|indicator/i.test(text),
      hasMojibake: new RegExp(["\\u00c2", "\\u00c3", "\\ufffd", "R\\u00c2", "Q\\u00c2", "f\\u00c2"].join("|")).test(text),
      hasSmartplsEquivalenceClaim: /identical to SmartPLS|equivalent to SmartPLS|same as SmartPLS/i.test(text),
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
    overview: await readTab(page, "Overview", "01_overview_findings.png"),
    measurement: await readTab(page, "Measurement", "02_measurement_findings.png"),
    structural: await readTab(page, "Structural", "03_structural_findings.png"),
    validity: await readTab(page, "Validity", "04_validity_findings.png"),
    inference: await readTab(page, "Inference", "05_inference_findings.png"),
    prediction: await readTab(page, "Prediction", "06_prediction_findings.png"),
    groups: await readTab(page, "Groups", "07_groups_findings.png"),
    diagnostics: await readTab(page, "Diagnostics", "08_diagnostics_findings.png"),
    interpretation: await readTab(page, "Interpretation", "09_interpretation_checklist.png"),
  };

  await selectTab(page, "Structural");
  await page.evaluate(() => {
    const section = Array.from(document.querySelectorAll(".result-table-section"))
      .find((item) => item.querySelector(".result-section-title strong")?.textContent?.includes("Inner VIF"))
      ?? Array.from(document.querySelectorAll(".result-table-section"))
        .find((item) => !item.querySelector("tr.result-path-row"));
    const row = section?.querySelector("table tbody tr");
    if (!row) throw new Error("No result row available for detail smoke");
    row.click();
  });
  await page.waitForSelector(".result-row-detail", { timeout: 5_000 });
  const rowDetail = await screenshot(page, "10_row_detail_interpretation.png");
  const rowDetailState = await page.evaluate(() => ({
    count: document.querySelectorAll(".result-row-detail").length,
    text: Array.from(document.querySelectorAll(".result-row-detail")).map((item) => item.textContent ?? "").join("\n"),
  }));

  await page.evaluate(() => window.__QUICKPLS_SMOKE__.setView("reports"));
  await page.waitForSelector(".publication-workspace", { timeout: 10_000 });
  await page.getByLabel("Include interpretation notes").check();
  const reportWording = await screenshot(page, "11_report_interpretation_option.png");
  const reportOptionChecked = await page.getByLabel("Include interpretation notes").isChecked();

  await page.setViewportSize({ width: 1280, height: 800 });
  const desktop1280 = await screenshot(page, "12_results_1280.png");

  const checklist = {
    overview_has_computed_findings: tabs.overview.findingCards >= 3,
    measurement_has_loading_finding: tabs.measurement.findingCards >= 1 && tabs.measurement.hasValueSpecificCopy,
    structural_has_path_r2_vif_finding: tabs.structural.findingCards >= 2 && tabs.structural.hasValueSpecificCopy,
    validity_has_value_specific_panel: tabs.validity.findingPanels >= 1 && tabs.validity.hasValueSpecificCopy,
    inference_covers_unavailable_or_bootstrap: tabs.inference.findingPanels >= 1 && tabs.inference.hasValueSpecificCopy,
    prediction_has_guidance: tabs.prediction.findingPanels >= 1,
    groups_has_state_or_payload_guidance: tabs.groups.findingPanels >= 1,
    diagnostics_has_method_payload_guidance: tabs.diagnostics.findingPanels >= 1,
    interpretation_has_prioritized_checklist: ["Must address before reporting", "Recommended checks", "Optional advanced checks"].every((label) => tabs.interpretation.checklists.includes(label)),
    interpretation_has_report_wording: tabs.interpretation.reportWording,
    copy_controls_present: Object.values(tabs).some((tab) => tab.copyInterpretationButtons > 0),
    row_detail_uses_values: rowDetailState.count > 0 && /Selected row interpretation/.test(rowDetailState.text) && /\d/.test(rowDetailState.text),
    report_has_interpretation_notes_option: reportOptionChecked,
    screenshots_written: Object.values(tabs).every((tab) => Boolean(tab.screenshot)) && Boolean(rowDetail) && Boolean(reportWording) && Boolean(desktop1280),
    no_mojibake_or_overclaim: Object.values(tabs).every((tab) => !tab.hasMojibake && !tab.hasSmartplsEquivalenceClaim),
    no_console_errors: errors.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v1.5.6 result-specific interpretation smoke",
    passed: Object.values(checklist).every(Boolean),
    checklist,
    tabs,
    screenshots: { rowDetail, reportWording, desktop1280 },
    rowDetailState,
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
