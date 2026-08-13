import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v2111", "home-data");
const OUTPUT = path.join(RESULTS, "v2111_home_data_mockup_smoke.json");
const PORT = 53214;
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
  await page.waitForTimeout(300);
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
    const root = currentView === "welcome"
      ? document.querySelector(".home-v211-workspace")
      : document.querySelector(".data-v211-workspace");
    return {
      bodyText,
      screenshotTextLength: bodyText.length,
      horizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
      hasWorkspacePage: Boolean(root?.classList.contains("qpls2-page-shell")),
      hasPageHeader: Boolean(root?.querySelector(".qpls2-workspace-hero .qpls2-page-title")),
      hasPrimaryAction: Boolean(root?.querySelector(".qpls2-primary-action")),
      hasPanel: Boolean(root?.querySelector(".qpls2-panel")),
      hasMetricCard: Boolean(root?.querySelector(".qpls2-metric-card")),
      hasInlineNotice: Boolean(root?.querySelector(".qpls2-inline-notice")) || currentView === "welcome",
      homeHasCommandPanel: Boolean(document.querySelector(".home-v211-command-panel")),
      homeHasWorkflow: Boolean(document.querySelector(".home-v2-workflow")),
      dataHasImportPanel: Boolean(document.querySelector(".data-v2-import.qpls2-panel")),
      dataHasQualityMetrics: document.querySelectorAll(".data-v211-workspace .qpls2-metric-card").length >= 6,
      dataHasPreview: Boolean(document.querySelector(".data-v211-workspace .data-workbench")),
      hasCurrentVersionLabel: bodyText.includes("v2.1.1 home/data mockup alignment"),
      hasMojibake: /RÃ|Â²|Ãƒ|â€/.test(bodyText),
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

  captures.push(await capture(page, "welcome", "1440x900"));
  captures.push(await capture(page, "data", "1440x900"));
  await page.setViewportSize({ width: 1280, height: 800 });
  captures.push(await capture(page, "welcome", "1280x800"));
  captures.push(await capture(page, "data", "1280x800"));

  for (const captureResult of captures) {
    const workspace = captureResult.view === "welcome" ? "Home" : "Data";
    const metrics = captureResult.metrics;
    if (!metrics.hasWorkspacePage) issues.push(issue(`${workspace}-001`, "high", workspace, "Missing qpls2 page shell", "The workspace is not composed through the shared v2.1 WorkspacePage primitive."));
    if (!metrics.hasPageHeader) issues.push(issue(`${workspace}-002`, "high", workspace, "Missing v2 page header", "The first viewport does not expose the shared PageHeader title."));
    if (!metrics.hasPrimaryAction) issues.push(issue(`${workspace}-003`, "high", workspace, "Missing primary action", "The workspace needs one clear first-viewport primary action."));
    if (!metrics.hasPanel) issues.push(issue(`${workspace}-004`, "high", workspace, "Missing v2 panel", "The workspace should use shared Panel sections for decision surfaces."));
    if (!metrics.hasMetricCard) issues.push(issue(`${workspace}-005`, "medium", workspace, "Missing metric cards", "Home/Data should expose compact workspace facts or data-quality metrics."));
    if (metrics.horizontalOverflow > 6) issues.push(issue(`${workspace}-006`, "high", workspace, "Horizontal page overflow", `The page is ${metrics.horizontalOverflow}px wider than the viewport.`));
    if (!metrics.hasCurrentVersionLabel) issues.push(issue(`${workspace}-007`, "medium", workspace, "Stale version label", "The visible top bar should show v2.1.1 home/data mockup alignment."));
    if (metrics.hasMojibake) issues.push(issue(`${workspace}-008`, "high", workspace, "Mojibake visible", "Rendered text contains corrupted characters."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`${workspace}-009`, "high", workspace, "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
  }

  const checklist = {
    screenshots_written: captures.every((item) => Boolean(item.screenshot)),
    home_1440_and_1280_captured: captures.filter((item) => item.view === "welcome").length === 2,
    data_1440_and_1280_captured: captures.filter((item) => item.view === "data").length === 2,
    home_command_panel_present: captures.some((item) => item.metrics.homeHasCommandPanel),
    home_workflow_present: captures.some((item) => item.metrics.homeHasWorkflow),
    data_import_panel_present: captures.some((item) => item.metrics.dataHasImportPanel),
    data_quality_metrics_present: captures.some((item) => item.metrics.dataHasQualityMetrics),
    data_preview_present: captures.some((item) => item.metrics.dataHasPreview),
    no_console_errors: errors.length === 0,
    no_visual_issues: issues.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.1.1 Home/Data mockup alignment smoke",
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
