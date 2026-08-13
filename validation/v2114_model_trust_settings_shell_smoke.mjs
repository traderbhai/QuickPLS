import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v2114", "model-trust-settings");
const OUTPUT = path.join(RESULTS, "v2114_model_trust_settings_shell_smoke.json");
const PORT = 53217;
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
  await page.waitForTimeout(450);
  await page.evaluate(() => {
    document.querySelector(".page-host")?.scrollTo({ top: 0, left: 0 });
    window.scrollTo({ top: 0, left: 0 });
  });
  await page.waitForTimeout(100);
}

async function capture(page, view, viewport) {
  await setView(page, view);
  const screenshot = path.join(ARTIFACTS, `${viewport}_${view}.png`);
  await page.screenshot({ path: screenshot, fullPage: true });
  const metrics = await page.evaluate((currentView) => {
    const bodyText = document.body.innerText;
    const modelRoot = document.querySelector(".model-v214-workspace");
    const trustRoot = document.querySelector(".trust-v214-workspace");
    const settingsRoot = document.querySelector(".settings-v214-workspace");
    const root = currentView === "models" ? modelRoot : currentView === "trust" ? trustRoot : settingsRoot;
    return {
      textLength: bodyText.length,
      hasCurrentVersionLabel: bodyText.includes("v2.1.4 model/trust/settings shell alignment"),
      hasMojibake: /RÃƒÆ’Ã†â€™|RÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²|RÃƒâ€šÃ‚Â²|RÂ²|ÃƒÆ’Ã¢â‚¬Å¡/.test(bodyText),
      hasSmartPlsEquivalenceClaim: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(bodyText),
      modelHasShell: Boolean(modelRoot),
      modelHasToolbar: Boolean(document.querySelector(".model-v214-toolbar")),
      modelHasOverlayStatus: Boolean(document.querySelector(".model-v214-overlay-status")),
      modelKeepsReactFlow: Boolean(document.querySelector(".react-flow")),
      trustHasShell: Boolean(trustRoot?.classList.contains("qpls2-page-shell")),
      trustHasHeader: Boolean(trustRoot?.querySelector(".qpls2-workspace-hero .qpls2-page-title")),
      trustHasPanels: (trustRoot?.querySelectorAll(".qpls2-panel").length ?? 0) >= 3,
      trustHasMetricCards: (trustRoot?.querySelectorAll(".qpls2-metric-card").length ?? 0) >= 4,
      trustHasResearchTables: (trustRoot?.querySelectorAll(".research-table-shell").length ?? 0) >= 3,
      settingsHasShell: Boolean(settingsRoot?.classList.contains("qpls2-page-shell")),
      settingsHasHeader: Boolean(settingsRoot?.querySelector(".qpls2-workspace-hero .qpls2-page-title")),
      settingsHasPanels: (settingsRoot?.querySelectorAll(".qpls2-panel").length ?? 0) >= 2,
      settingsHasMetricCards: (settingsRoot?.querySelectorAll(".qpls2-metric-card").length ?? 0) >= 4,
      rootPresent: Boolean(root),
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

  for (const view of ["models", "trust", "settings"]) captures.push(await capture(page, view, "1440x900"));
  await page.setViewportSize({ width: 1280, height: 800 });
  for (const view of ["models", "trust", "settings"]) captures.push(await capture(page, view, "1280x800"));

  for (const captureResult of captures) {
    const workspace = captureResult.view;
    const metrics = captureResult.metrics;
    if (!metrics.rootPresent) issues.push(issue(`${workspace}-001`, "high", workspace, "Missing workspace root", "The selected view did not render its expected v2.1.4 root marker."));
    if (!metrics.hasCurrentVersionLabel) issues.push(issue(`${workspace}-002`, "medium", workspace, "Stale version label", "The visible top bar should show v2.1.4 model/trust/settings shell alignment."));
    if (metrics.hasMojibake) issues.push(issue(`${workspace}-003`, "high", workspace, "Mojibake visible", "Rendered text contains corrupted characters."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`${workspace}-004`, "high", workspace, "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
    if (workspace === "models" && (!metrics.modelHasShell || !metrics.modelHasToolbar || !metrics.modelHasOverlayStatus || !metrics.modelKeepsReactFlow)) {
      issues.push(issue("models-005", "high", "models", "Model shell incomplete", "Model should retain React Flow and expose v2.1.4 shell, toolbar, and overlay markers."));
    }
    if (workspace === "trust" && (!metrics.trustHasShell || !metrics.trustHasHeader || !metrics.trustHasPanels || !metrics.trustHasMetricCards || !metrics.trustHasResearchTables)) {
      issues.push(issue("trust-005", "high", "trust", "Trust Center primitive coverage incomplete", "Trust Center should use v2 page, panel, metric, and research table primitives."));
    }
    if (workspace === "settings" && (!metrics.settingsHasShell || !metrics.settingsHasHeader || !metrics.settingsHasPanels || !metrics.settingsHasMetricCards)) {
      issues.push(issue("settings-005", "high", "settings", "Settings primitive coverage incomplete", "Settings should use v2 page, panel, and metric primitives."));
    }
  }

  const checklist = {
    screenshots_written: captures.every((item) => Boolean(item.screenshot)),
    model_captured_at_two_viewports: captures.filter((item) => item.view === "models").length === 2,
    trust_captured_at_two_viewports: captures.filter((item) => item.view === "trust").length === 2,
    settings_captured_at_two_viewports: captures.filter((item) => item.view === "settings").length === 2,
    model_shell_markers_present: captures.some((item) => item.metrics.modelHasShell && item.metrics.modelHasToolbar && item.metrics.modelHasOverlayStatus && item.metrics.modelKeepsReactFlow),
    trust_primitives_present: captures.some((item) => item.metrics.trustHasShell && item.metrics.trustHasPanels && item.metrics.trustHasMetricCards && item.metrics.trustHasResearchTables),
    settings_primitives_present: captures.some((item) => item.metrics.settingsHasShell && item.metrics.settingsHasPanels && item.metrics.settingsHasMetricCards),
    no_console_errors: errors.length === 0,
    no_visual_issues: issues.length === 0,
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.1.4 Model/Trust/Settings shell smoke",
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
