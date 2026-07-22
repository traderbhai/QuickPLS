import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v150", "researcher-ux");
const OUTPUT = path.join(RESULTS, "v150_researcher_ux_smoke.json");
const PORT = 53150;
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
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

  await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true, bubbles: true })));
  shots.commandPalette = path.join(ARTIFACTS, "01_command_palette.png");
  await page.screenshot({ path: shots.commandPalette, fullPage: true });
  const commandText = await bodyText(page);
  await page.keyboard.press("Escape");

  await page.keyboard.press("?");
  shots.shortcuts = path.join(ARTIFACTS, "02_shortcuts.png");
  await page.screenshot({ path: shots.shortcuts, fullPage: true });
  const shortcutText = await bodyText(page);
  await page.keyboard.press("Escape");

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("analyses"));
  await page.waitForTimeout(250);
  await page.locator(".what-will-run-card").scrollIntoViewIfNeeded().catch(() => {});
  shots.methodSummary = path.join(ARTIFACTS, "03_method_what_will_run.png");
  await page.screenshot({ path: shots.methodSummary, fullPage: true });
  const methodText = await bodyText(page);

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("runs"));
  await page.waitForTimeout(250);
  shots.results = path.join(ARTIFACTS, "04_results_headlines.png");
  await page.screenshot({ path: shots.results, fullPage: true });
  const resultsText = await bodyText(page);

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("reports"));
  await page.waitForTimeout(250);
  shots.reports = path.join(ARTIFACTS, "05_export_stepper.png");
  await page.screenshot({ path: shots.reports, fullPage: true });
  const reportText = await bodyText(page);

  await page.evaluate(() => {
    const modelButton = [...document.querySelectorAll(".nav-rail button")].find((button) => button.textContent?.includes("Model"));
    if (modelButton instanceof HTMLElement) modelButton.click();
  });
  await page.waitForTimeout(500);
  await page.evaluate(() => {
    const variableButton = [...document.querySelectorAll(".explorer-tabs button")].find((button) => button.textContent?.includes("Variables"));
    if (variableButton instanceof HTMLElement) variableButton.click();
  });
  await page.locator(".prefix-chip-row").first().waitFor({ state: "visible", timeout: 5000 }).catch(() => {});
  shots.prefixGroups = path.join(ARTIFACTS, "06_prefix_groups.png");
  await page.screenshot({ path: shots.prefixGroups, fullPage: true });
  const explorerText = await bodyText(page);

  const checklist = {
    command_palette_visible: includesAll(commandText, ["Quick actions", "Import dataset", "Arrange like SmartPLS", "Publication preview mode"]),
    shortcut_overlay_visible: includesAll(shortcutText, ["Keyboard shortcuts", "Ctrl+K", "Path tool", "Fit diagram to view"]),
    method_summary_visible: includesAll(methodText, ["What will run", "Bootstrap", "Permutation", "Scope"]),
    result_headlines_visible: includesAll(resultsText, ["Selected run", "Strongest R²", "Export current table", "Warnings"]),
    export_stepper_visible: includesAll(reportText, ["Select run", "Choose diagram style", "Preview figure", "Export tables and SVG"]),
    prefix_groups_visible: includesAll(explorerText, ["Prefix groups"]),
    screenshots_written: Object.values(shots).length === 6,
    no_console_errors: errors.length === 0,
  };
  const report = {
    schema_version: 1,
    target: "QuickPLS v1.5.0 researcher UX smoke",
    passed: Object.values(checklist).every(Boolean),
    checklist,
    screenshots: shots,
    errors,
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  server.kill();
}
