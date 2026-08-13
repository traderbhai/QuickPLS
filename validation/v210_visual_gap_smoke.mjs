import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v210", "visual-gap");
const OUTPUT = path.join(RESULTS, "v210_visual_gap_smoke.json");
const PORT = 53210;
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

async function capture(page, view, label, suffix) {
  await setView(page, view);
  const screenshot = path.join(ARTIFACTS, `${suffix}_${view}.png`);
  await page.screenshot({ path: screenshot, fullPage: true });
  const metrics = await page.evaluate(() => {
    const rect = (selector) => {
      const el = document.querySelector(selector);
      if (!el) return null;
      const box = el.getBoundingClientRect();
      return { x: box.x, y: box.y, width: box.width, height: box.height };
    };
    const bodyText = document.body.innerText;
    const pageHost = document.querySelector(".page-host");
    const activeRail = document.querySelector(".nav-item.active")?.textContent?.trim() ?? "";
    const horizontalOverflow = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth;
    const visibleHeader = Boolean(document.querySelector(".page-heading h1, .page-heading-pro h1, .qpls2-page-title, .model-v2-canvas, .trust-v2-hero, .settings-v2-workspace"));
    const appShell = rect(".app-shell");
    const commandBar = rect(".command-bar");
    const workflowStrip = rect(".workflow-strip");
    const rail = rect(".nav-rail");
    const page = rect(".qpls2-workspace, .model-v2-canvas, .trust-v2-workspace, .settings-v2-workspace");
    return {
      bodyText,
      activeRail,
      horizontalOverflow,
      visibleHeader,
      appShell,
      commandBar,
      workflowStrip,
      rail,
      page,
      hasV2Panel: Boolean(document.querySelector(".qpls2-panel, .ui-card, .home-v2-hero, .data-v2-import, .setup-v2-hero, .run-v2-hero, .results-v2-command-center, .results-v2-layout, .report-v2-hero, .trust-v2-hero, .settings-v2-workspace, .model-v2-explorer, .model-v2-inspector")),
      hasPrimaryAction: Boolean(document.querySelector(".qpls2-primary-action, .run-button")),
      hasTrustEntry: /Why trust|Trust Center|Method confidence/i.test(bodyText),
      hasReportability: /Reportability|What to inspect|Interpretation|Scope/i.test(bodyText),
      hasMojibake: /RÃ|RÂ²|Ãƒ|Â/.test(bodyText),
      hasSmartPlsEquivalenceClaim: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(bodyText),
    };
  });
  return { view, label, screenshot, metrics };
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

  for (const [view, label] of WORKSPACES) {
    captures[view] = await capture(page, view, label, "1440x900");
  }

  await page.setViewportSize({ width: 1280, height: 800 });
  captures.desktop1280 = {};
  for (const [view, label] of [["welcome", "Home"], ["data", "Data"], ["models", "Model"], ["runs", "Results"], ["reports", "Report"], ["trust", "Trust"]]) {
    captures.desktop1280[view] = await capture(page, view, label, "1280x800");
  }

  const allCaptures = [
    ...Object.values(captures).filter((item) => item?.metrics),
    ...Object.values(captures.desktop1280 ?? {}),
  ];
  for (const item of allCaptures) {
    const { view, label, metrics } = item;
    if (!metrics.visibleHeader) issues.push(issue(`V210-${view}-001`, "high", label, "Workspace header or primary surface is not visible", "The workspace did not expose a recognizable v2 page header or primary canvas at first paint.", "Make each workspace open with its title or primary surface visible at 1440x900 and 1280x800."));
    if (!metrics.hasV2Panel) issues.push(issue(`V210-${view}-002`, "medium", label, "V2 panel language is missing", "The screen does not expose shared v2 panels or the v2 model shell.", "Route this workspace through shared qpls2 panel primitives or documented model-shell equivalents."));
    if (metrics.horizontalOverflow > 6) issues.push(issue(`V210-${view}-003`, "high", label, "Horizontal page overflow", `The rendered page is ${metrics.horizontalOverflow}px wider than the viewport.`, "Move secondary controls into menus, reduce fixed widths, or constrain wide tables inside internal scroll regions."));
    if (metrics.hasMojibake) issues.push(issue(`V210-${view}-004`, "high", label, "Mojibake visible in UI", "The rendered text contains corrupted characters.", "Replace corrupted text and add a source/screenshot check for the affected label."));
    if (metrics.hasSmartPlsEquivalenceClaim) issues.push(issue(`V210-${view}-005`, "high", label, "Unsupported SmartPLS equivalence claim", "The rendered screen implies equivalence to SmartPLS.", "Replace with bounded QuickPLS validation wording."));
  }

  const trustScreens = [captures.analyses, captures.runs, captures.reports, captures.trust].filter(Boolean);
  if (!trustScreens.every((item) => item.metrics.hasTrustEntry)) {
    issues.push(issue("V210-global-006", "medium", "Global", "Trust entry is not consistently visible", "Setup, Results, Report, and Trust should expose an obvious confidence/scope route.", "Add a compact 'Why trust this?' or Method Confidence entry point in every evidence-heavy workspace."));
  }
  if (!captures.runs.metrics.hasReportability) {
    issues.push(issue("V210-results-007", "high", "Results", "Results lacks interpretation/reportability entry points", "The Results workspace should make interpretation and reportability visible without hunting.", "Keep reportability checklist, interpretation controls, and row-detail guidance in the first Results viewport."));
  }

  const checklist = {
    screenshots_written: allCaptures.every((item) => Boolean(item.screenshot)),
    all_primary_workspaces_captured: WORKSPACES.every(([view]) => Boolean(captures[view]?.screenshot)),
    desktop_1280_subset_captured: ["welcome", "data", "models", "runs", "reports", "trust"].every((view) => Boolean(captures.desktop1280?.[view]?.screenshot)),
    active_rail_matches_workspace: WORKSPACES.every(([view, label]) => captures[view]?.metrics.activeRail.includes(label)),
    no_console_errors: errors.length === 0,
    no_high_severity_visual_gaps: !issues.some((item) => item.severity === "high"),
    no_mojibake: allCaptures.every((item) => !item.metrics.hasMojibake),
    no_smartpls_equivalence_claim: allCaptures.every((item) => !item.metrics.hasSmartPlsEquivalenceClaim),
  };

  const result = {
    schema_version: 1,
    target: "QuickPLS v2.0.10 visual gap smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
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
