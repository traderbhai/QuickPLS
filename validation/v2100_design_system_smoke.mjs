import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v2100", "design-system");
const OUTPUT = path.join(RESULTS, "v2100_design_system_smoke.json");
const PORT = 53213;
const URL = `http://127.0.0.1:${PORT}/`;
const WORKSPACES = [
  ["welcome", "Home"],
  ["data", "Data"],
  ["models", "Model"],
  ["analyses", "Setup"],
  ["run", "Run"],
  ["runs", "Results"],
  ["reports", "Report"],
  ["trust", "Trust"],
  ["settings", "Settings"],
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
  await page.waitForTimeout(275);
  await page.evaluate(() => {
    document.querySelector(".page-host")?.scrollTo({ top: 0, left: 0 });
    window.scrollTo({ top: 0, left: 0 });
  });
  await page.waitForTimeout(75);
}

async function capture(page, view, suffix) {
  await setView(page, view);
  const screenshot = path.join(ARTIFACTS, `${suffix}_${view}.png`);
  await page.screenshot({ path: screenshot, fullPage: true });
  const metrics = await page.evaluate(() => {
    const bodyText = document.body.innerText;
    const horizontalOverflow = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth;
    return {
      bodyText,
      activeRail: document.querySelector(".nav-item.active")?.textContent?.trim() ?? "",
      horizontalOverflow,
      visibleHeader: Boolean(document.querySelector(".qpls2-page-title, .page-heading h1, .page-heading-pro h1, .model-v2-canvas, .data-v2-workspace h1, .run-v2-workspace h1, .results-v2-workspace h1, .report-v2-workspace h1, .trust-v2-hero, .settings-v2-workspace")),
      hasV2Workspace: Boolean(document.querySelector(".qpls2-workspace, .model-v2-canvas, .home-v2-workspace, .data-v2-workspace, .run-v2-workspace, .results-v2-workspace, .report-v2-workspace, .trust-v2-workspace, .settings-v2-workspace")),
      hasPanel: Boolean(document.querySelector(".qpls2-panel, .qpls2-design-panel, .ui-card, .home-v2-current, .home-v2-workflow, .data-v2-import, .data-v2-quality, .data-preview-panel, .setup-v2-hero, .setup-v2-method-browser, .run-v2-hero, .run-v2-readiness, .results-v2-command-center, .results-v2-layout, .report-v2-hero, .report-v2-panel, .trust-v2-hero, .model-v2-explorer, .model-v2-inspector")),
      hasCurrentVersionLabel: bodyText.includes("v2.1.0 design system foundation"),
      hasMojibake: /RÃƒ|RÃ‚Â²|ÃƒÆ’|Ã‚/.test(bodyText),
      hasSmartPlsEquivalenceClaim: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(bodyText),
    };
  });
  return { view, screenshot, metrics };
}

function issue(id, severity, workspace, title, detail, recommendation) {
  return { id, severity, workspace, title, detail, recommendation };
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const captures = {};
  const issues = [];

  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

  for (const [view] of WORKSPACES) captures[view] = await capture(page, view, "1440x900");

  await page.setViewportSize({ width: 1280, height: 800 });
  captures.desktop1280 = {};
  for (const [view] of [["welcome"], ["data"], ["runs"], ["reports"], ["settings"]]) {
    captures.desktop1280[view] = await capture(page, view, "1280x800");
  }

  const settingsChecks = await page.evaluate(() => {
    window.__QUICKPLS_SMOKE__?.setView("settings");
    const bodyText = document.body.innerText;
    return {
      hasWorkspacePage: Boolean(document.querySelector(".qpls2-page-shell")),
      hasDesignPanel: Boolean(document.querySelector(".settings-v2-design-system .qpls2-design-panel, .settings-v2-design-system")),
      hasMetricCards: document.querySelectorAll(".qpls2-metric-card").length >= 4,
      hasInlineNotices: document.querySelectorAll(".qpls2-inline-notice").length >= 2,
      hasCommandGroups: document.querySelectorAll(".qpls2-command-group").length >= 2,
      hasToolbarButtons: document.querySelectorAll(".qpls2-toolbar-button").length >= 4,
      hasLocalDisabledReason: bodyText.includes("Import a raw dataset before running."),
      hasPrimitiveCopy: bodyText.includes("Design system foundation"),
    };
  });

  const allCaptures = [
    ...Object.values(captures).filter((item) => item?.metrics),
    ...Object.values(captures.desktop1280 ?? {}),
  ];
  for (const item of allCaptures) {
    const { view, metrics } = item;
    const workspace = WORKSPACES.find(([id]) => id === view)?.[1] ?? view;
    if (!metrics.visibleHeader) issues.push(issue(`V2100-${view}-001`, "high", workspace, "Workspace header or primary surface is not visible", "The workspace did not expose a recognizable v2 title or primary canvas at first paint.", "Use the shared WorkspacePage/PageHeader primitives or the documented model-shell equivalent."));
    if (!metrics.hasV2Workspace) issues.push(issue(`V2100-${view}-002`, "high", workspace, "V2 workspace shell is missing", "The screen does not expose a v2 workspace or model canvas surface.", "Route workspaces through qpls2 shell primitives."));
    if (!metrics.hasPanel) issues.push(issue(`V2100-${view}-003`, "high", workspace, "V2 panel language is missing", "The screen does not expose shared panels or documented model shell panels.", "Compose screen-specific layouts from the shared qpls2 panel primitives."));
    if (metrics.horizontalOverflow > 6) issues.push(issue(`V2100-${view}-004`, "high", workspace, "Horizontal page overflow", `The rendered page is ${metrics.horizontalOverflow}px wider than the viewport.`, "Constrain wide tables/canvases to internal scroll regions."));
    if (!metrics.hasCurrentVersionLabel) issues.push(issue(`V2100-${view}-005`, "medium", workspace, "Version label is stale", "The top bar does not show v2.1.0 design system foundation.", "Update visible release/status text before shipping the milestone."));
    if (metrics.hasMojibake) issues.push(issue(`V2100-${view}-006`, "high", workspace, "Mojibake visible in UI", "The rendered text contains corrupted characters.", "Replace corrupted text and keep source/screenshot checks active."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`V2100-${view}-007`, "high", workspace, "Unsupported SmartPLS equivalence claim", "The rendered screen implies equivalence to SmartPLS.", "Use bounded QuickPLS validation wording only."));
  }

  for (const [key, value] of Object.entries(settingsChecks)) {
    if (!value) issues.push(issue(`V2100-settings-${key}`, "high", "Settings", `Design-system check failed: ${key}`, "The Settings design-system preview is missing an expected primitive or sample.", "Keep the v2.1 primitive preview visible until broader workspace rebuilds consume the primitives."));
  }

  const checklist = {
    screenshots_written: allCaptures.every((item) => Boolean(item.screenshot)),
    all_primary_workspaces_captured: WORKSPACES.every(([view]) => Boolean(captures[view]?.screenshot)),
    desktop_1280_subset_captured: ["welcome", "data", "runs", "reports", "settings"].every((view) => Boolean(captures.desktop1280?.[view]?.screenshot)),
    active_rail_matches_workspace: WORKSPACES.every(([view, label]) => captures[view]?.metrics.activeRail.includes(label)),
    no_console_errors: errors.length === 0,
    no_visual_gaps: issues.length === 0,
    design_system_preview_complete: Object.values(settingsChecks).every(Boolean),
    no_mojibake: allCaptures.every((item) => !item.metrics.hasMojibake),
    current_version_label_visible: allCaptures.every((item) => item.metrics.hasCurrentVersionLabel),
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.1.0 design system foundation smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    settingsChecks,
    issues,
    errors,
    captures,
  };
  await fs.writeFile(OUTPUT, JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ ...result, captures: Object.fromEntries(Object.entries(captures).map(([key, value]) => [key, value?.screenshot ?? Object.fromEntries(Object.entries(value).map(([innerKey, innerValue]) => [innerKey, innerValue.screenshot]))])) }, null, 2));
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
