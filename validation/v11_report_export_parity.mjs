import { spawn, spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "v11", "report-parity");
const PORT = 5212;
const PREVIEW_URL = `http://127.0.0.1:${PORT}/`;
const OUTPUT = path.join(RESULTS, "v11_report_export_parity.json");

await mkdir(ARTIFACTS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], {
  cwd: ROOT,
  windowsHide: true,
  stdio: ["ignore", "pipe", "pipe"],
});

let logs = "";
server.stdout.on("data", (data) => { logs += data.toString(); });
server.stderr.on("data", (data) => { logs += data.toString(); });

async function waitForUrl(url, deadlineMs = 45_000) {
  const start = Date.now();
  while (Date.now() - start < deadlineMs) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(2_000) });
      if (response.ok) return;
    } catch {
      // Keep polling.
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Timed out waiting for Vite preview. Logs: ${logs.slice(-2000)}`);
}

function normalize(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

let browser;
try {
  await waitForUrl(PREVIEW_URL);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, acceptDownloads: true });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });

  await page.goto(`${PREVIEW_URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  await page.locator(".nav-item").filter({ hasText: /^Report$/ }).click();
  await page.waitForTimeout(500);

  const preview = await page.locator(".publication-preview-shell .diagram-preview svg").evaluate((svg) => ({
    text: svg.textContent ?? "",
    outer: svg.outerHTML,
  }));
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.locator(".publication-preview-heading button").filter({ hasText: /Export SVG/ }).click(),
  ]);
  const downloadedPath = path.join(ARTIFACTS, "quickpls-publication-diagram.svg");
  await download.saveAs(downloadedPath);
  const exported = await readFile(downloadedPath, "utf-8");
  await page.screenshot({ path: path.join(ARTIFACTS, "report-parity.png"), fullPage: true });

  const expectedLabels = ["0.842", "0.874", "0.902", "0.913", "0.403", "0.327", "0.544"];
  const previewText = normalize(preview.text);
  const exportText = normalize(exported.replace(/<[^>]+>/g, " "));
  const checklist = {
    preview_svg_present: preview.outer.includes("<svg"),
    exported_svg_file_written: exported.includes("<svg") && exported.length > 500,
    preview_has_expected_labels: expectedLabels.every((label) => previewText.includes(label)),
    export_has_expected_labels: expectedLabels.every((label) => exportText.includes(label)),
    preview_and_export_have_same_key_labels: expectedLabels.every((label) => previewText.includes(label) === exportText.includes(label)),
    export_omits_edit_chrome: !/Trash|Assign dataset variable|Choose variable|React Flow/i.test(exported),
    no_r_squared_mojibake: !["R\u00c3", "R\u00c2", "R\ufffd"].some((bad) => previewText.includes(bad) || exportText.includes(bad)),
  };
  const report = {
    schema_version: 1,
    target: "QuickPLS v1.1 report SVG parity",
    viewport: { width: 1440, height: 900 },
    passed: errors.length === 0 && Object.values(checklist).every(Boolean),
    checklist,
    errors,
    artifacts: {
      screenshot: path.join(ARTIFACTS, "report-parity.png"),
      exported_svg: downloadedPath,
    },
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
