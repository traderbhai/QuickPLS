import { execFileSync, spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v152", "data-workspace");
const OUTPUT = path.join(RESULTS, "v152_data_workspace_smoke.json");
const PORT = 53153;
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

async function bodyText(page) {
  return page.locator("body").innerText();
}

async function screenshot(page, name) {
  const file = path.join(ARTIFACTS, name);
  await page.screenshot({ path: file, fullPage: true });
  return file;
}

function includesAll(text, needles) {
  const haystack = text.toLocaleLowerCase();
  return needles.every((needle) => haystack.includes(needle.toLocaleLowerCase()));
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

  await page.getByRole("button", { name: /^Data:/i }).click();
  await page.waitForTimeout(250);
  shots.raw = await screenshot(page, "01_raw_data_workspace.png");
  const rawText = await bodyText(page);

  await page.getByLabel("Data type").selectOption("covariance");
  await page.waitForTimeout(150);
  shots.covariance = await screenshot(page, "02_covariance_mode.png");
  const covarianceText = await bodyText(page);

  await page.getByRole("spinbutton", { name: "Sample size" }).fill("12");
  await page.waitForTimeout(150);
  shots.matrixReady = await screenshot(page, "03_matrix_ready.png");
  const matrixReadyText = await bodyText(page);

  await page.getByLabel("Data type").selectOption("correlation");
  await page.waitForTimeout(150);
  shots.correlation = await screenshot(page, "04_correlation_mode.png");
  const correlationText = await bodyText(page);

  await page.getByLabel("Data type").selectOption("raw");
  await page.getByLabel("Search variables in data preview").fill("comp");
  await page.waitForTimeout(150);
  shots.search = await screenshot(page, "05_variable_search.png");
  const searchText = await bodyText(page);

  await page.getByLabel("Search variables in data preview").fill("");
  await page.getByLabel("Filter variables by metadata").selectOption("ordinal");
  await page.waitForTimeout(150);
  shots.filter = await screenshot(page, "06_variable_filter.png");
  const filterText = await bodyText(page);

  await page.getByLabel("Filter variables by metadata").selectOption("all");
  await page.waitForTimeout(100);
  await page.getByRole("button", { name: "COMP2" }).click();
  await page.waitForTimeout(150);
  shots.metadata = await screenshot(page, "07_metadata_selected_column.png");
  const metadataText = await bodyText(page);

  await page.setViewportSize({ width: 1280, height: 800 });
  await page.waitForTimeout(150);
  shots.desktop1280 = await screenshot(page, "08_desktop_1280.png");

  const prefixText = rawText;
  const checklist = {
    raw_mode_has_import_source_quality_preview: includesAll(rawText, ["Import source", "Data quality", "Data preview"]),
    duplicate_fixture_language_removed: !rawText.includes("Validation fixture"),
    sample_dataset_action_present: rawText.includes("Load Sample Dataset"),
    quality_cards_present: includesAll(rawText, ["Rows", "Variables", "Missing cells", "Nonnumeric", "Constant columns", "Header issues"]),
    small_sample_warning_visible: rawText.includes("Small sample"),
    prefix_bridge_present: includesAll(prefixText, ["Create constructs from prefixes", "COMP -> 3 indicators", "LIKE -> 2 indicators"]),
    covariance_mode_has_persistent_requirement: includesAll(covarianceText, ["Covariance matrix", "Sample size", "study sample size of at least 2", "Current loaded dataset preview"]),
    matrix_ready_keeps_mode_clear: includesAll(matrixReadyText, ["Current loaded dataset preview", "Showing 9 of 9 columns"]),
    correlation_mode_has_guidance: includesAll(correlationText, ["Correlation matrix", "Matrix imports require"]),
    search_filters_columns: searchText.includes("Showing 3 of 9 columns") && !searchText.includes("LIKE1"),
    scale_filter_filters_columns: filterText.includes("Showing 0 of 9 columns") || filterText.includes("Showing 9 of 9 columns"),
    metadata_selection_visible: includesAll(metadataText, ["COMP2", "Selected column metadata", "Reset draft"]),
    screenshots_written: Object.values(shots).length === 8,
    no_console_errors: errors.length === 0,
  };
  const result = { schema_version: 1, target: "QuickPLS v1.5.2 Data workspace smoke", passed: Object.values(checklist).every(Boolean), checklist, screenshots: shots, errors };
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
