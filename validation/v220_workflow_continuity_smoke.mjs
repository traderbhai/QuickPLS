import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v220", "workflow-continuity");
const OUTPUT = path.join(RESULTS, "v220_workflow_continuity_smoke.json");
const PORT = 53220;
const URL = `http://127.0.0.1:${PORT}/`;

const VIEWS = [
  { id: "welcome", label: "Home", expectedCoach: "home-ready-setup" },
  { id: "data", label: "Data", expectedCoach: "data-continue-model" },
  { id: "analyses", label: "Setup", expectedCoach: "setup-needs-work" },
  { id: "run", label: "Run", expectedCoach: "run-blocked" },
  { id: "runs", label: "Results", expectedCoach: "results-empty" },
  { id: "reports", label: "Report", expectedCoach: "report-needs-run" },
  { id: "trust", label: "Trust Center", expectedCoach: "trust-scope" },
  { id: "settings", label: "Settings", expectedCoach: "settings-preferences" },
];

const VIEWPORTS = [
  { id: "1440x900", width: 1440, height: 900 },
  { id: "1280x800", width: 1280, height: 800 },
];

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
  await page.waitForTimeout(100);
}

function issue(id, severity, workspace, title, detail) {
  return { id, severity, workspace, title, detail };
}

async function capture(page, view, viewport) {
  await setView(page, view.id);
  const screenshot = path.join(ARTIFACTS, `${viewport.id}_${view.id}.png`);
  await page.screenshot({ path: screenshot, fullPage: true });
  const metrics = await page.evaluate((currentView) => {
    const bodyText = document.body.innerText;
    const coach = document.querySelector(".workspace-coach");
    const coachRect = coach?.getBoundingClientRect();
    const pageHost = document.querySelector(".page-host");
    const actions = coach ? Array.from(coach.querySelectorAll("button")).map((button) => button.textContent?.trim() ?? "") : [];
    return {
      hasVersionLabel: bodyText.includes("v2.2.0 workflow clarity"),
      coachPresent: Boolean(coach),
      coachId: coach?.getAttribute("data-coach-id") ?? "",
      coachAria: coach?.getAttribute("aria-label") ?? "",
      coachVisible: Boolean(coachRect && coachRect.width > 100 && coachRect.height > 20),
      coachActions: actions,
      hasPrimaryAction: actions.length >= 1,
      pageHostAtTop: !pageHost || pageHost.scrollTop === 0,
      hasFrameworkOverlay: /Vite|React Error|Unhandled Runtime Error|Internal server error/i.test(bodyText),
      hasMojibake: /RÃ|RÂ|Â²/.test(bodyText),
      hasSmartPlsEquivalenceClaim: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(bodyText),
    };
  }, view);
  return { view: view.id, viewport: viewport.id, screenshot, metrics };
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

  for (const viewport of VIEWPORTS) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    for (const view of VIEWS) captures.push(await capture(page, view, viewport));
  }

  for (const item of captures) {
    const view = VIEWS.find((candidate) => candidate.id === item.view);
    const { viewport, metrics } = item;
    const prefix = `${viewport}-${item.view}`;
    if (!metrics.hasVersionLabel) issues.push(issue(`${prefix}-version`, "medium", item.view, "Version label mismatch", "The top bar should show v2.2.0 workflow clarity."));
    if (!metrics.coachPresent) issues.push(issue(`${prefix}-coach`, "high", item.view, "Workflow coach missing", "The workspace should render the workflow coach."));
    if (metrics.coachAria !== "Workflow coach") issues.push(issue(`${prefix}-coach-aria`, "medium", item.view, "Workflow coach accessible name missing", "The coach needs a stable aria-label."));
    if (metrics.coachId !== view.expectedCoach) issues.push(issue(`${prefix}-coach-id`, "medium", item.view, "Unexpected workflow coach message", `Expected ${view.expectedCoach}, saw ${metrics.coachId || "nothing"}.`));
    if (!metrics.coachVisible) issues.push(issue(`${prefix}-coach-visible`, "high", item.view, "Workflow coach not visible", "The coach should be visible near the top of the workspace."));
    if (!metrics.hasPrimaryAction) issues.push(issue(`${prefix}-coach-action`, "high", item.view, "Workflow coach action missing", "The coach should expose a next action."));
    if (!metrics.pageHostAtTop) issues.push(issue(`${prefix}-scroll`, "medium", item.view, "Workspace did not open at top", "Workspace switching should reset the main page scroll to top."));
    if (metrics.hasFrameworkOverlay) issues.push(issue(`${prefix}-overlay`, "high", item.view, "Framework overlay visible", "The page appears to show a framework/runtime error overlay."));
    if (metrics.hasMojibake) issues.push(issue(`${prefix}-mojibake`, "high", item.view, "Mojibake visible", "Rendered text includes corrupted encoding artifacts."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`${prefix}-smartpls`, "high", item.view, "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
  }

  const checklist = {
    screenshots_written: captures.every((item) => Boolean(item.screenshot)),
    all_expected_captures_written: captures.length === VIEWS.length * VIEWPORTS.length,
    every_view_has_two_viewports: VIEWS.every((view) => captures.filter((item) => item.view === view.id).length === 2),
    no_console_errors: errors.length === 0,
    no_visual_issues: issues.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.0 workflow continuity smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    viewports: VIEWPORTS,
    views: VIEWS,
    checklist,
    issues,
    errors,
    captures,
  };
  await fs.writeFile(OUTPUT, JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ ...result, captures: captures.map((item) => ({ view: item.view, viewport: item.viewport, screenshot: item.screenshot, coachId: item.metrics.coachId })) }, null, 2));
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
