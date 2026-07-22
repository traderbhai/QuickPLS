import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v14", "frontend-success");
const OUTPUT = path.join(RESULTS, "v14_frontend_success_smoke.json");
const PORT = 53147;
const URL = `http://127.0.0.1:${PORT}/`;

await mkdir(ARTIFACTS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
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
  throw new Error(`Timed out waiting for Vite preview. ${logs.slice(-1000)}`);
}

async function pageText(page) {
  return page.locator("body").innerText();
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const shots = {};
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });

  shots.onboarding = path.join(ARTIFACTS, "01_onboarding.png");
  await page.screenshot({ path: shots.onboarding, fullPage: true });
  const onboardingText = await pageText(page);

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("models"));
  await page.waitForTimeout(250);
  shots.semDesigner = path.join(ARTIFACTS, "02_sem_designer.png");
  await page.screenshot({ path: shots.semDesigner, fullPage: true });
  const designerText = await pageText(page);
  const designerMetrics = await page.evaluate(() => ({
    toolbarOverflow: [...document.querySelectorAll(".canvas-toolbar")].some((el) => el.scrollWidth > el.clientWidth + 2),
    hasViewMenu: Boolean([...document.querySelectorAll("button")].find((button) => button.textContent?.trim() === "View")),
    hasResultsMenu: Boolean([...document.querySelectorAll("button")].find((button) => button.textContent?.trim() === "Results")),
  }));

  await page.locator(".canvas-toolbar-primary").getByRole("button", { name: /^View$/ }).click();
  shots.viewMenu = path.join(ARTIFACTS, "03_view_large_model_menu.png");
  await page.screenshot({ path: shots.viewMenu, fullPage: true });
  const viewMenuText = await pageText(page);

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("analyses"));
  await page.waitForTimeout(250);
  shots.methodSetup = path.join(ARTIFACTS, "04_method_setup.png");
  await page.screenshot({ path: shots.methodSetup, fullPage: true });
  const methodText = await pageText(page);

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("runs"));
  await page.waitForTimeout(250);
  shots.results = path.join(ARTIFACTS, "05_results_workspace.png");
  await page.screenshot({ path: shots.results, fullPage: true });
  const resultsText = await pageText(page);

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("reports"));
  await page.waitForTimeout(250);
  shots.reports = path.join(ARTIFACTS, "06_reports_export.png");
  await page.screenshot({ path: shots.reports, fullPage: true });
  const reportText = await pageText(page);

  await page.setViewportSize({ width: 1280, height: 800 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("models"));
  await page.waitForTimeout(250);
  shots.desktop1280 = path.join(ARTIFACTS, "07_desktop_1280.png");
  await page.screenshot({ path: shots.desktop1280, fullPage: true });

  const checklist = {
    onboarding_has_desktop_first_actions: ["Start new project", "Open existing project", "Open demo project", "Import dataset"].every((needle) => onboardingText.includes(needle)),
    sem_designer_toolbar_has_core_actions: ["Select", "Pan", "Construct", "Path", "Cov", "Arrange", "Fit", "Validate", "View", "Results"].every((needle) => designerText.includes(needle)),
    toolbar_has_no_horizontal_overflow_at_1440: !designerMetrics.toolbarOverflow,
    view_menu_has_large_model_controls: ["Collapse measurement indicators", "Isolate selected neighborhood", "Fit selected", "Lock layout"].every((needle) => viewMenuText.includes(needle)),
    method_setup_has_basic_expert_and_presets: ["Basic", "Expert", "Standard PLS-SEM", "PLS + Bootstrap", "MICOM + MGA"].every((needle) => methodText.includes(needle)),
    results_workspace_has_researcher_tabs: ["Summary", "Measurement Model", "Structural Model", "Inference", "Diagnostics", "Comparison"].every((needle) => resultsText.includes(needle)),
    reports_have_publication_presets: ["Thesis appendix", "Journal figure", "Journal tables", "Presentation", "Full reproducibility report"].every((needle) => reportText.includes(needle)),
    no_r2_mojibake_visible: ![onboardingText, designerText, viewMenuText, methodText, resultsText, reportText].some((body) => body.includes("RÂ²")),
    screenshots_written: Object.values(shots).length === 7,
    no_console_errors: errors.length === 0,
  };
  const report = {
    schema_version: 1,
    target: "QuickPLS v1.4 frontend success program smoke",
    passed: Object.values(checklist).every(Boolean),
    checklist,
    screenshots: shots,
    errors,
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  server.kill();
}
