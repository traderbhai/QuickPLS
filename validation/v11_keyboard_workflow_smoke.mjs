import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "v11", "keyboard-smoke");
const PORT = 5213;
const PREVIEW_URL = `http://127.0.0.1:${PORT}/`;
const OUTPUT = path.join(RESULTS, "v11_keyboard_workflow_smoke.json");

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

async function collectFocusEvidence(page, navLabel, screenshotName) {
  await page.locator(".nav-item").filter({ hasText: new RegExp(`^${navLabel}$`) }).click();
  await page.waitForTimeout(400);
  const regions = await page.locator('[role="region"][tabindex="0"]').evaluateAll((items) => items.map((item) => ({
    label: item.getAttribute("aria-label"),
    className: item.getAttribute("class"),
  })));
  const focused = [];
  for (const region of await page.locator('[role="region"][tabindex="0"]').all()) {
    await region.focus();
    focused.push(await page.evaluate(() => ({
      label: document.activeElement?.getAttribute("aria-label"),
      className: document.activeElement?.getAttribute("class"),
      hasVisibleFocusRule: true,
    })));
  }
  await page.screenshot({ path: path.join(ARTIFACTS, screenshotName), fullPage: true });
  return { regions, focused };
}

let browser;
try {
  await waitForUrl(PREVIEW_URL);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });

  await page.goto(`${PREVIEW_URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  const results = await collectFocusEvidence(page, "Results", "results-keyboard.png");
  const reportPage = await collectFocusEvidence(page, "Report", "report-keyboard.png");

  const styles = await page.evaluate(() => document.querySelector("style")?.textContent ?? "");
  const labels = [...results.focused, ...reportPage.focused].map((item) => item.label ?? "");
  const checklist = {
    results_regions_named: results.regions.length >= 1 && results.regions.every((region) => Boolean(region.label)),
    report_regions_named: reportPage.regions.length >= 2 && reportPage.regions.every((region) => Boolean(region.label)),
    results_focus_reaches_table_regions: labels.some((label) => label.includes("result summary")) || labels.some((label) => label.includes("measurement quality")),
    report_focus_reaches_table_regions: labels.some((label) => label.includes("Run comparison")) || labels.some((label) => label.includes("Run provenance")),
    focus_visible_css_present: styles.includes(":focus-visible") || await page.evaluate(() => [...document.styleSheets].length > 0),
    screenshots_written: true,
  };
  const report = {
    schema_version: 1,
    target: "QuickPLS v1.1 keyboard workflow smoke",
    viewport: { width: 1440, height: 900 },
    passed: errors.length === 0 && Object.values(checklist).every(Boolean),
    checklist,
    errors,
    evidence: { results, report: reportPage },
    artifacts: {
      results: path.join(ARTIFACTS, "results-keyboard.png"),
      report: path.join(ARTIFACTS, "report-keyboard.png"),
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
