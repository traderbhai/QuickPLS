import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const PORT = 5188;
const PREVIEW_URL = `http://127.0.0.1:${PORT}/`;
const OUTPUT = path.join(RESULTS, "v091_diagram_visual_smoke.json");
const SCREENSHOT = path.join(RESULTS, "v091_diagram_default_1440x900.png");

await mkdir(RESULTS, { recursive: true });

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
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 2_000);
      const response = await fetch(url, { signal: controller.signal });
      clearTimeout(timeout);
      if (response.ok) return;
    } catch {
      // Keep polling until the preview server is ready.
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Timed out waiting for Vite preview. Logs: ${logs.slice(-2000)}`);
}

let browser;
try {
  await waitForUrl(PREVIEW_URL);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(PREVIEW_URL, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForTimeout(3_000);
  const metrics = await page.evaluate(() => ({
    latent: document.querySelectorAll(".latent-node").length,
    indicators: document.querySelectorAll(".indicator-node").length,
    measurementEdges: [...document.querySelectorAll(".react-flow__edge")].filter((edge) => edge.classList.contains("measurement-edge")).length,
    toolbarHasSem: [...document.querySelectorAll("select")].some((select) => select.textContent?.includes("SEM diagram")),
    helpButton: Boolean(document.querySelector("[title=\"Diagram legend\"]")),
  }));
  await page.screenshot({ path: SCREENSHOT, fullPage: true });
  const passed = metrics.latent > 0
    && metrics.indicators > 0
    && metrics.measurementEdges > 0
    && metrics.toolbarHasSem
    && metrics.helpButton
    && errors.length === 0;
  const report = { schema_version: 1, target: "v0.9.1 diagram visual smoke", passed, metrics, errors, screenshot: SCREENSHOT };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  server.kill();
}
