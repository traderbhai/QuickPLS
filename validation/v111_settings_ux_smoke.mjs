import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const OUTPUT = path.join(RESULTS, "v111_settings_ux_smoke.json");
const ARTIFACTS = path.join(RESULTS, "screens", "v111", "settings-ux");
const PORT = 5312;
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

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });

  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.locator(".nav-item").filter({ hasText: /^Validate$/ }).click();
  await page.waitForTimeout(400);
  const before = await page.evaluate(() => ({
    has_guidance: Boolean(document.querySelector(".settings-guidance")?.textContent?.includes("Recommended defaults")),
    advanced_closed: document.querySelector(".advanced-settings")?.hasAttribute("open") === false,
    bootstrap_hidden_when_closed: !document.body.textContent?.includes("Studentized inner replicates") || document.querySelector(".advanced-settings")?.hasAttribute("open") === false,
    readiness_present: Boolean(document.querySelector(".readiness-panel")),
  }));
  await page.locator(".advanced-settings summary").click();
  await page.waitForTimeout(150);
  const after = await page.evaluate(() => ({
    advanced_open: document.querySelector(".advanced-settings")?.hasAttribute("open") === true,
    bootstrap_visible: Boolean([...document.querySelectorAll("label")].find((label) => label.textContent?.includes("Bootstrap replicates"))),
    seed_visible: Boolean([...document.querySelectorAll("label")].find((label) => label.textContent?.includes("Random seed"))),
    workers_visible: Boolean([...document.querySelectorAll("label")].find((label) => label.textContent?.includes("Workers"))),
  }));
  const screenshot = path.join(ARTIFACTS, "settings-progressive-disclosure.png");
  await page.screenshot({ path: screenshot, fullPage: true });

  const checklist = {
    recommended_defaults_guidance_visible: before.has_guidance,
    advanced_settings_collapsed_by_default: before.advanced_closed,
    readiness_panel_visible: before.readiness_present,
    advanced_settings_expand_to_resampling_controls: after.advanced_open && after.bootstrap_visible && after.seed_visible && after.workers_visible,
    screenshot_written: true,
  };
  const report = { schema_version: 1, target: "QuickPLS v1.1.1 method settings progressive disclosure", passed: errors.length === 0 && Object.values(checklist).every(Boolean), checklist, errors, evidence: { before, after, screenshot } };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
