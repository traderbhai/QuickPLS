import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v2112", "setup-run");
const OUTPUT = path.join(RESULTS, "v2112_setup_run_mockup_smoke.json");
const PORT = 53215;
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
    const root = currentView === "analyses"
      ? document.querySelector(".setup-v212-workspace")
      : document.querySelector(".run-v212-workspace");
    return {
      bodyText,
      screenshotTextLength: bodyText.length,
      horizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
      hasWorkspacePage: Boolean(root?.classList.contains("qpls2-page-shell")),
      hasPageHeader: Boolean(root?.querySelector(".qpls2-workspace-hero .qpls2-page-title")),
      hasPanel: Boolean(root?.querySelector(".qpls2-panel")),
      hasPrimaryAction: Boolean(root?.querySelector(".qpls2-primary-action")),
      hasStatusBadge: Boolean(root?.querySelector(".ui-status-badge")),
      setupHasMethodBrowser: Boolean(document.querySelector(".setup-v212-workspace .setup-v2-method-browser")),
      setupHasRequirements: Boolean(document.querySelector(".setup-v212-workspace .setup-v2-requirements")),
      setupHasCalculationPreview: Boolean(document.querySelector(".setup-v212-workspace .setup-v2-preview")),
      setupHasRunCommand: /Run now|Run selected method/.test(bodyText),
      runHasReadiness: Boolean(document.querySelector(".run-v212-workspace .run-v2-readiness-panel")),
      runHasOutputPreview: Boolean(document.querySelector(".run-v212-workspace .run-v2-output-preview")),
      runHasExecutionPlan: Boolean(document.querySelector(".run-v212-workspace .run-v2-execution-plan")),
      runHasRunCommand: bodyText.includes("Run selected method"),
      hasCurrentVersionLabel: bodyText.includes("v2.1.2 setup/run mockup alignment"),
      hasMojibake: /RÃƒ|RÃ‚Â²|RÂ²|ÃƒÆ’|Ã‚|Ã¢â‚¬/.test(bodyText),
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

  captures.push(await capture(page, "analyses", "1440x900"));
  captures.push(await capture(page, "run", "1440x900"));
  await page.setViewportSize({ width: 1280, height: 800 });
  captures.push(await capture(page, "analyses", "1280x800"));
  captures.push(await capture(page, "run", "1280x800"));

  for (const captureResult of captures) {
    const workspace = captureResult.view === "analyses" ? "Setup" : "Run";
    const metrics = captureResult.metrics;
    if (!metrics.hasWorkspacePage) issues.push(issue(`${workspace}-001`, "high", workspace, "Missing qpls2 page shell", "The workspace is not composed through the shared v2.1 WorkspacePage primitive."));
    if (!metrics.hasPageHeader) issues.push(issue(`${workspace}-002`, "high", workspace, "Missing v2 page header", "The first viewport does not expose the shared PageHeader title."));
    if (!metrics.hasPanel) issues.push(issue(`${workspace}-003`, "high", workspace, "Missing v2 panel", "The workspace should use shared Panel sections for decision surfaces."));
    if (!metrics.hasPrimaryAction) issues.push(issue(`${workspace}-004`, "high", workspace, "Missing primary action", "The workspace needs one clear calculation launch action."));
    if (!metrics.hasStatusBadge) issues.push(issue(`${workspace}-005`, "medium", workspace, "Missing status badge", "Setup/Run should expose readiness or scope state in a compact badge."));
    if (metrics.horizontalOverflow > 6) issues.push(issue(`${workspace}-006`, "high", workspace, "Horizontal page overflow", `The page is ${metrics.horizontalOverflow}px wider than the viewport.`));
    if (!metrics.hasCurrentVersionLabel) issues.push(issue(`${workspace}-007`, "medium", workspace, "Stale version label", "The visible top bar should show v2.1.2 setup/run mockup alignment."));
    if (metrics.hasMojibake) issues.push(issue(`${workspace}-008`, "high", workspace, "Mojibake visible", "Rendered text contains corrupted characters."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`${workspace}-009`, "high", workspace, "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
  }

  const checklist = {
    screenshots_written: captures.every((item) => Boolean(item.screenshot)),
    setup_1440_and_1280_captured: captures.filter((item) => item.view === "analyses").length === 2,
    run_1440_and_1280_captured: captures.filter((item) => item.view === "run").length === 2,
    setup_method_browser_present: captures.some((item) => item.metrics.setupHasMethodBrowser),
    setup_requirements_present: captures.some((item) => item.metrics.setupHasRequirements),
    setup_calculation_preview_present: captures.some((item) => item.metrics.setupHasCalculationPreview),
    setup_run_command_present: captures.some((item) => item.metrics.setupHasRunCommand),
    run_readiness_present: captures.some((item) => item.metrics.runHasReadiness),
    run_output_preview_present: captures.some((item) => item.metrics.runHasOutputPreview),
    run_execution_plan_present: captures.some((item) => item.metrics.runHasExecutionPlan),
    run_command_present: captures.some((item) => item.metrics.runHasRunCommand),
    no_console_errors: errors.length === 0,
    no_visual_issues: issues.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.1.2 Setup/Run mockup alignment smoke",
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
