import { spawn, spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v111", "export-parity");
const OUTPUT = path.join(RESULTS, "v111_report_export_parity.json");
const PORT = 5313;
const URL = `http://127.0.0.1:${PORT}/`;

await mkdir(ARTIFACTS, { recursive: true });
const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
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

function svgStats(svg) {
  const text = svg.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
  return {
    text,
    svg_present: svg.includes("<svg"),
    latent_count: (svg.match(/class="[^"]*latent/g) ?? []).length,
    indicator_count: (svg.match(/class="[^"]*indicator/g) ?? []).length,
    path_count: (svg.match(/structural-edge|class="path"/g) ?? []).length,
    has_r2: /R(&#178;|²|\u00b2)|R2/.test(svg),
    has_edit_chrome: /Trash|React Flow|Assign dataset variable|Choose variable|canvas-toolbar/i.test(svg),
    has_mojibake: /RÂ|RÃ|R�/.test(svg),
  };
}

async function exportCurrentSvg(page, name) {
  await page.locator(".nav-item").filter({ hasText: /^Report$/ }).click();
  await page.waitForTimeout(300);
  const preview = await page.locator(".publication-preview-shell .diagram-preview svg").evaluate((svg) => svg.outerHTML);
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.locator(".publication-preview-heading button").filter({ hasText: /Export SVG/ }).click(),
  ]);
  const downloadedPath = path.join(ARTIFACTS, `${name}.svg`);
  await download.saveAs(downloadedPath);
  const exported = await readFile(downloadedPath, "utf-8");
  await page.screenshot({ path: path.join(ARTIFACTS, `${name}.png`), fullPage: true });
  const previewStats = svgStats(preview);
  const exportStats = svgStats(exported);
  return {
    name,
    downloadedPath,
    preview: previewStats,
    exported: exportStats,
    same_core_counts: previewStats.latent_count === exportStats.latent_count && previewStats.indicator_count === exportStats.indicator_count,
    no_edit_chrome: !previewStats.has_edit_chrome && !exportStats.has_edit_chrome,
    no_mojibake: !previewStats.has_mojibake && !exportStats.has_mojibake,
  };
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, acceptDownloads: true });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });

  const cases = [];
  cases.push(await exportCurrentSvg(page, "model-only-current"));
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  cases.push(await exportCurrentSvg(page, "completed-result-current"));
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadDiagramFixture("large"));
  cases.push(await exportCurrentSvg(page, "large-model-only-current"));
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadDiagramFixture("formative"));
  cases.push(await exportCurrentSvg(page, "formative-model-only-current"));
  await page.locator("label").filter({ hasText: /Diagram layout/ }).locator("select").selectOption("tidy_publication");
  cases.push(await exportCurrentSvg(page, "formative-model-only-tidy"));

  const checklist = {
    all_cases_have_svg: cases.every((item) => item.preview.svg_present && item.exported.svg_present),
    all_cases_have_matching_core_counts: cases.every((item) => item.same_core_counts),
    completed_result_has_r2: cases.find((item) => item.name === "completed-result-current")?.exported.has_r2 === true,
    all_cases_omit_edit_chrome: cases.every((item) => item.no_edit_chrome),
    all_cases_avoid_r2_mojibake: cases.every((item) => item.no_mojibake),
  };
  const report = { schema_version: 1, target: "QuickPLS v1.1.1 broad report/export parity", passed: errors.length === 0 && Object.values(checklist).every(Boolean), checklist, errors, cases };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
