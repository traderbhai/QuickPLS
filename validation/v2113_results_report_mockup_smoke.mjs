import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v2113", "results-report");
const OUTPUT = path.join(RESULTS, "v2113_results_report_mockup_smoke.json");
const PORT = 53216;
const URL = `http://127.0.0.1:${PORT}/`;

await fs.mkdir(ARTIFACTS, { recursive: true });

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

async function setView(page, view) {
  await page.evaluate((nextView) => window.__QUICKPLS_SMOKE__?.setView(nextView), view);
  await page.waitForTimeout(350);
  await page.evaluate(() => {
    document.querySelector(".page-host")?.scrollTo({ top: 0, left: 0 });
    window.scrollTo({ top: 0, left: 0 });
  });
  await page.waitForTimeout(75);
}

async function capture(page, view, viewport) {
  await setView(page, view);
  const screenshot = path.join(ARTIFACTS, `${viewport}_${view}.png`);
  await page.screenshot({ path: screenshot, fullPage: true });
  const metrics = await page.evaluate((currentView) => {
    const bodyText = document.body.innerText;
    const root = currentView === "runs"
      ? document.querySelector(".results-v213-workspace")
      : document.querySelector(".report-v213-workspace");
    const commandCenter = root?.querySelector(".results-v213-command-center, .report-v213-command-center");
    return {
      screenshotTextLength: bodyText.length,
      commandCenterOverflow: commandCenter ? commandCenter.scrollWidth - commandCenter.clientWidth : 0,
      hasWorkspacePage: Boolean(root?.classList.contains("qpls2-page-shell")),
      hasPageHeader: Boolean(root?.querySelector(".qpls2-workspace-hero .qpls2-page-title")),
      hasPanel: Boolean(root?.querySelector(".qpls2-panel")),
      hasStatusBadge: Boolean(root?.querySelector(".ui-status-badge")),
      resultsHasCommandCenter: Boolean(document.querySelector(".results-v213-workspace .results-v213-command-center")),
      resultsHasTabs: Boolean(document.querySelector(".results-v213-workspace .results-section-nav")),
      resultsHasToolStack: Boolean(document.querySelector(".results-v213-workspace .results-tool-stack")),
      resultsHasEmptyOrRunContext: Boolean(document.querySelector(".results-v213-workspace .empty-state, .results-v213-workspace .results-v213-run-context")),
      reportHasHero: Boolean(document.querySelector(".report-v213-workspace .report-v213-hero")),
      reportHasCommandCenter: Boolean(document.querySelector(".report-v213-workspace .report-v213-command-center")),
      reportHasSettings: Boolean(document.querySelector(".report-v213-workspace .report-v213-settings")),
      reportHasPreview: Boolean(document.querySelector(".report-v213-workspace .report-v213-preview")),
      reportHasExportActions: Boolean(document.querySelector(".report-v213-workspace .report-v213-export-actions")),
      hasCurrentVersionLabel: bodyText.includes("v2.1.3 results/report mockup alignment"),
      hasMojibake: /RÃƒÆ’|RÃƒâ€šÃ‚Â²|RÃ‚Â²|ÃƒÆ’Ã†â€™|Ãƒâ€š|ÃƒÂ¢Ã¢â€šÂ¬/.test(bodyText),
      hasSmartPlsEquivalenceClaim: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(bodyText),
    };
  }, view);
  return { view, viewport, screenshot, metrics };
}

function issue(id, severity, workspace, title, detail) {
  return { id, severity, workspace, title, detail };
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const captures = [];
  const issues = [];
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

  captures.push(await capture(page, "runs", "1440x900"));
  captures.push(await capture(page, "reports", "1440x900"));
  await page.setViewportSize({ width: 1280, height: 800 });
  captures.push(await capture(page, "runs", "1280x800"));
  captures.push(await capture(page, "reports", "1280x800"));

  for (const captureResult of captures) {
    const workspace = captureResult.view === "runs" ? "Results" : "Report";
    const metrics = captureResult.metrics;
    if (!metrics.hasWorkspacePage) issues.push(issue(`${workspace}-001`, "high", workspace, "Missing qpls2 page shell", "The workspace is not composed through the shared v2.1 WorkspacePage primitive."));
    if (!metrics.hasPageHeader) issues.push(issue(`${workspace}-002`, "high", workspace, "Missing v2 page header", "The first viewport does not expose the shared PageHeader title."));
    if (!metrics.hasPanel) issues.push(issue(`${workspace}-003`, "high", workspace, "Missing v2 panel", "The workspace should use shared Panel sections."));
    if (!metrics.hasStatusBadge) issues.push(issue(`${workspace}-004`, "medium", workspace, "Missing status badge", "Results/Report should expose compact scope or selection state."));
    if (metrics.commandCenterOverflow > 6) issues.push(issue(`${workspace}-005`, "high", workspace, "Command center overflow", `The main command panel is ${metrics.commandCenterOverflow}px wider than its viewport.`));
    if (!metrics.hasCurrentVersionLabel) issues.push(issue(`${workspace}-006`, "medium", workspace, "Stale version label", "The visible top bar should show v2.1.3 results/report mockup alignment."));
    if (metrics.hasMojibake) issues.push(issue(`${workspace}-007`, "high", workspace, "Mojibake visible", "Rendered text contains corrupted characters."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`${workspace}-008`, "high", workspace, "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
  }

  const checklist = {
    screenshots_written: captures.every((item) => Boolean(item.screenshot)),
    results_1440_and_1280_captured: captures.filter((item) => item.view === "runs").length === 2,
    report_1440_and_1280_captured: captures.filter((item) => item.view === "reports").length === 2,
    results_command_center_present: captures.some((item) => item.metrics.resultsHasCommandCenter),
    results_tabs_present: captures.some((item) => item.metrics.resultsHasTabs),
    results_tool_stack_present: captures.some((item) => item.metrics.resultsHasToolStack),
    results_empty_or_context_present: captures.some((item) => item.metrics.resultsHasEmptyOrRunContext),
    report_hero_present: captures.some((item) => item.metrics.reportHasHero),
    report_command_center_present: captures.some((item) => item.metrics.reportHasCommandCenter),
    report_settings_present: captures.some((item) => item.metrics.reportHasSettings),
    report_preview_present: captures.some((item) => item.metrics.reportHasPreview),
    report_export_actions_present: captures.some((item) => item.metrics.reportHasExportActions),
    no_console_errors: errors.length === 0,
    no_visual_issues: issues.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.1.3 Results/Report mockup alignment smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    issues,
    errors,
    captures,
  };
  await fs.writeFile(OUTPUT, JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ ...result, captures: captures.map((item) => ({ view: item.view, viewport: item.viewport, screenshot: item.screenshot })) }, null, 2));
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
