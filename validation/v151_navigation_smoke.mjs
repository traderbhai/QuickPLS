import { execFileSync, spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v151", "navigation");
const OUTPUT = path.join(RESULTS, "v151_navigation_smoke.json");
const PORT = 53151;
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
  throw new Error(`Timed out waiting for Vite preview. ${logs.slice(-1000)}`);
}

async function text(page) {
  return page.locator("body").innerText();
}

function hasAll(body, needles) {
  const haystack = body.toLocaleLowerCase();
  return needles.every((needle) => haystack.includes(needle.toLocaleLowerCase()));
}

async function openRail(page, label, filename) {
  await page.getByRole("button", { name: new RegExp(`^${label}:`, "i") }).click();
  await page.waitForTimeout(250);
  const screenshot = path.join(ARTIFACTS, filename);
  await page.screenshot({ path: screenshot, fullPage: true });
  return { screenshot, body: await text(page) };
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
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

  const home = await openRail(page, "Home", "01_home.png");
  shots.home = home.screenshot;
  const data = await openRail(page, "Data", "02_data.png");
  shots.data = data.screenshot;
  const model = await openRail(page, "Model", "03_model.png");
  shots.model = model.screenshot;
  const setup = await openRail(page, "Setup", "04_setup.png");
  shots.setup = setup.screenshot;
  const run = await openRail(page, "Run", "05_run.png");
  shots.run = run.screenshot;

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  const results = await openRail(page, "Results", "06_results_summary.png");
  shots.results = results.screenshot;
  await page.getByRole("tab", { name: "Groups" }).click();
  await page.waitForTimeout(250);
  shots.resultsGroups = path.join(ARTIFACTS, "07_results_groups.png");
  await page.screenshot({ path: shots.resultsGroups, fullPage: true });
  const groupsText = await text(page);

  const report = await openRail(page, "Report", "08_report.png");
  shots.report = report.screenshot;

  const navLabels = await page.locator(".nav-rail .nav-item span").evaluateAll((items) => items.map((item) => item.textContent ?? ""));
  const checklist = {
    primary_rail_sequence: JSON.stringify(navLabels) === JSON.stringify(["Home", "Data", "Model", "Setup", "Run", "Results", "Report"]),
    no_primary_validate_or_groups: !navLabels.includes("Validate") && !navLabels.includes("Groups"),
    home_current_project_card: hasAll(home.body, ["Home", "Current", "Open project"]),
    data_next_step: hasAll(data.body, ["Data preview", "Open Model Designer"]),
    model_canvas_available: hasAll(model.body, ["Select", "Path", "Validate"]),
    setup_readiness_available: hasAll(setup.body, ["Setup", "Analysis readiness", "Standard PLS-SEM"]),
    run_workspace_available: hasAll(run.body, ["Run", "Before publication", "Open results"]),
    results_summary_available: hasAll(results.body, ["Results", "Overview", "Export current table"]),
    results_groups_tab_available: hasAll(groupsText, ["Groups and segmentation results", "Configure group workflow in Setup"]),
    report_workspace_available: hasAll(report.body, ["Publication report", "Export tables and SVG"]),
    screenshots_written: Object.values(shots).length === 8,
    no_console_errors: errors.length === 0,
  };
  const result = { schema_version: 1, target: "QuickPLS v1.5.1 navigation smoke", passed: Object.values(checklist).every(Boolean), checklist, navLabels, screenshots: shots, errors };
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
