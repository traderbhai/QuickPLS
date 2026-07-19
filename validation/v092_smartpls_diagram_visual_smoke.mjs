import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const PORT = 5192;
const PREVIEW_URL = `http://127.0.0.1:${PORT}/`;
const OUTPUT = path.join(RESULTS, "v092_smartpls_diagram_visual_smoke.json");
const SCREENSHOT = path.join(RESULTS, "v092_smartpls_result_1440x900.png");

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
  await page.selectOption("[aria-label=\"Diagram mode\"]", "smartpls_result");
  await page.waitForTimeout(1_200);
  const metrics = await page.evaluate(() => ({
    smartplsLatents: document.querySelectorAll(".smartpls-latent-node").length,
    smartplsIndicators: document.querySelectorAll(".smartpls-indicator-node").length,
    smartplsMeasurementEdges: document.querySelectorAll(".smartpls-measurement-edge, .react-flow__edge-path.smartpls-measurement-edge").length,
    smartplsStructuralEdges: document.querySelectorAll(".smartpls-structural-edge, .react-flow__edge-path.smartpls-structural-edge").length,
    gridVisible: Boolean(document.querySelector(".react-flow__background")),
    minimapVisible: Boolean(document.querySelector(".react-flow__minimap")),
    deleteButtons: document.querySelectorAll(".indicator-node button").length,
    warning: [...document.querySelectorAll(".canvas-tool-status.warning")].some((node) => node.textContent?.includes("Run or select a compatible result")),
    modeOption: [...document.querySelectorAll("select")].some((select) => select.textContent?.includes("Result diagram")),
  }));
  await page.screenshot({ path: SCREENSHOT, fullPage: true });
  const passed = metrics.smartplsLatents > 0
    && metrics.smartplsIndicators > 0
    && metrics.smartplsMeasurementEdges > 0
    && metrics.smartplsStructuralEdges > 0
    && !metrics.gridVisible
    && !metrics.minimapVisible
    && metrics.deleteButtons === 0
    && metrics.warning
    && metrics.modeOption
    && errors.length === 0;
  const report = { schema_version: 1, target: "v0.9.2 SmartPLS-like diagram visual smoke", passed, metrics, errors, screenshot: SCREENSHOT };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
