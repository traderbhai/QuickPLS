import { execFileSync, spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v153", "layout-copy");
const OUTPUT = path.join(RESULTS, "v153_layout_copy_smoke.json");
const PORT = 53154;
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

async function setView(page, view) {
  await page.evaluate((nextView) => window.__QUICKPLS_SMOKE__?.setView(nextView), view);
  await page.waitForTimeout(250);
}

async function screenshot(page, name, fullPage = true) {
  const file = path.join(ARTIFACTS, name);
  await page.screenshot({ path: file, fullPage });
  return file;
}

async function bodyText(page) {
  return page.locator("body").innerText();
}

function hasAnyCollision(text) {
  return [
    "Start new projectStart",
    "Import datasetCSV",
    "Missing dataset9",
    "Experimental scopeValidated",
    "Diagram exportSVG",
    "Table exportsRun",
    "RÂ²",
    "RÃ",
  ].some((needle) => text.includes(needle));
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const screenshots = {};
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

  await setView(page, "welcome");
  screenshots.home = await screenshot(page, "01_home_top.png");
  const homeText = await bodyText(page);

  await setView(page, "data");
  screenshots.dataTop = await screenshot(page, "02_data_top.png");
  const dataTopText = await bodyText(page);
  await page.getByRole("button", { name: "COMP1" }).click();
  await page.waitForTimeout(150);
  screenshots.dataPreview = await screenshot(page, "03_data_preview_profile.png");
  const dataPreviewText = await bodyText(page);

  await setView(page, "models");
  await page.getByText("Customer Satisfaction").first().click().catch(() => undefined);
  await page.waitForTimeout(250);
  screenshots.modelSelected = await screenshot(page, "04_model_selected_construct.png", false);
  const modelText = await bodyText(page);

  await setView(page, "analyses");
  screenshots.setupTop = await screenshot(page, "05_setup_top.png");
  const setupTopText = await bodyText(page);
  const pageHost = page.locator(".page-host");
  await pageHost.evaluate((el) => { el.scrollTop = 900; });
  await page.waitForTimeout(100);
  screenshots.setupPresets = await screenshot(page, "06_setup_preset_area.png");
  const setupPresetText = await bodyText(page);

  await setView(page, "run");
  screenshots.runBlocked = await screenshot(page, "07_run_blocked_state.png");
  const runText = await bodyText(page);

  await setView(page, "runs");
  screenshots.resultsEmpty = await screenshot(page, "08_results_empty_state.png");
  const resultsText = await bodyText(page);

  await setView(page, "reports");
  screenshots.reportSetup = await screenshot(page, "09_report_setup.png");
  const reportSetupText = await bodyText(page);
  await page.locator(".page-host").evaluate((el) => { el.scrollTop = 800; });
  await page.waitForTimeout(150);
  screenshots.reportPreview = await screenshot(page, "10_report_preview.png");
  const reportPreviewText = await bodyText(page);

  await page.setViewportSize({ width: 1280, height: 800 });
  await setView(page, "welcome");
  screenshots.desktop1280 = await screenshot(page, "11_desktop_1280_home.png");

  await page.locator(".page-host").evaluate((el) => { el.scrollTop = 600; });
  await setView(page, "data");
  const dataScrollTop = await page.locator(".page-host").evaluate((el) => el.scrollTop);

  const allText = [homeText, dataTopText, dataPreviewText, modelText, setupTopText, setupPresetText, runText, resultsText, reportSetupText, reportPreviewText].join("\n");
  const checklist = {
    home_cards_are_collision_free: !hasAnyCollision(homeText) && homeText.includes("Save project") && homeText.includes("Open project"),
    compact_blocker_chip_present: /1 blocker: (data|runtime)/i.test(allText),
    no_large_global_dataset_banner: !homeText.includes("This dataset is available for design/preview"),
    data_has_column_profile: dataPreviewText.includes("Column profile") && dataPreviewText.includes("Mean") && dataPreviewText.includes("Standard deviation"),
    model_hides_generic_path_labels: !modelText.includes("\nPath\nPath\n") && modelText.includes("Model-only diagram"),
    setup_status_copy_is_scoped: setupTopText.includes("Scope status") && !setupTopText.includes("Experimental scope") && setupTopText.includes("Validated for documented QuickPLS scope"),
    setup_group_workflows_are_progressive: !setupTopText.includes("MICOM + MGA setup") && setupPresetText.includes("Standard PLS-SEM"),
    run_disabled_reason_is_action_specific: runText.includes("Run disabled:"),
    results_empty_primary_matches_blocker: /(Open data|Open model)/i.test(resultsText) && resultsText.includes("Summary") && resultsText.includes("Diagnostics"),
    report_controls_are_aligned_and_specific: reportSetupText.includes("Export disabled:") && reportSetupText.includes("exporting result tables") && reportPreviewText.includes("Model-only SVG preview"),
    report_preview_visible: reportPreviewText.includes("Publication diagram preview") || reportPreviewText.includes("Model diagram"),
    scroll_resets_on_workspace_change: dataScrollTop === 0,
    screenshots_written: Object.values(screenshots).length === 11,
    no_text_collisions_or_mojibake: !hasAnyCollision(allText),
    no_console_errors: errors.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v1.5.3 layout, copy, and readiness polish smoke",
    passed: Object.values(checklist).every(Boolean),
    checklist,
    screenshots,
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
    try {
      execFileSync("powershell.exe", ["-NoProfile", "-Command", `(Get-NetTCPConnection -LocalPort ${PORT} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess -Unique) | ForEach-Object { Stop-Process -Id $_ -Force }`], { stdio: "ignore" });
    } catch {
      // Best-effort cleanup for detached Vite children.
    }
  } else {
    server.kill();
  }
}
