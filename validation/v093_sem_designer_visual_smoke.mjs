import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const SCREENS = path.join(RESULTS, "screens", "v093");
const PORT = 5193;
const PREVIEW_URL = `http://127.0.0.1:${PORT}/`;
const OUTPUT = path.join(RESULTS, "v093_sem_designer_visual_smoke.json");
const DEFAULT_SCREENSHOT = path.join(SCREENS, "v093_default_sem_canvas_1440x900.png");
const DRAG_SCREENSHOT = path.join(SCREENS, "v093_indicator_drag_1440x900.png");
const REPORT_SCREENSHOT = path.join(SCREENS, "v093_publication_preview_1440x900.png");

await mkdir(SCREENS, { recursive: true });

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
  await page.selectOption("[aria-label=\"Diagram mode\"]", "sem");
  await page.waitForTimeout(1_000);
  await page.screenshot({ path: DEFAULT_SCREENSHOT, fullPage: true });

  const firstIndicator = page.locator(".smartpls-indicator-node").first();
  const before = await firstIndicator.boundingBox();
  if (!before) throw new Error("No indicator node found on SEM canvas");
  await page.mouse.move(before.x + before.width / 2, before.y + before.height / 2);
  await page.mouse.down();
  await page.mouse.move(before.x + before.width / 2 + 115, before.y + before.height / 2 + 42, { steps: 12 });
  await page.mouse.up();
  await page.waitForTimeout(500);
  const after = await firstIndicator.boundingBox();
  await page.screenshot({ path: DRAG_SCREENSHOT, fullPage: true });
  const canvasMetrics = await page.evaluate(() => ({
    academicLatents: document.querySelectorAll(".smartpls-latent-node").length,
    academicIndicators: document.querySelectorAll(".smartpls-indicator-node").length,
    contextMenuCss: [...document.styleSheets].some((sheet) => {
      try { return [...sheet.cssRules].some((rule) => rule.cssText.includes("diagram-context-menu")); }
      catch { return false; }
    }),
  }));

  await page.click("text=Reports");
  await page.waitForTimeout(700);
  await page.selectOption("select:has(option[value=\"current_canvas\"])", "current_canvas").catch(() => undefined);
  await page.screenshot({ path: REPORT_SCREENSHOT, fullPage: true });

  const reportMetrics = await page.evaluate(() => ({
    reportSvg: Boolean(document.querySelector(".diagram-preview svg")),
    reportLayoutControl: [...document.querySelectorAll("select")].some((select) => select.textContent?.includes("Current canvas")),
  }));
  const metrics = { ...canvasMetrics, ...reportMetrics };
  const draggedIndicatorMoved = Boolean(after && (Math.abs(after.x - before.x) > 60 || Math.abs(after.y - before.y) > 20));
  const passed = metrics.academicLatents > 0
    && metrics.academicIndicators > 0
    && metrics.contextMenuCss
    && metrics.reportSvg
    && metrics.reportLayoutControl
    && draggedIndicatorMoved
    && errors.length === 0;
  const report = {
    schema_version: 1,
    target: "v0.9.3 professional SEM designer visual smoke",
    passed,
    metrics: { ...metrics, draggedIndicatorMoved },
    errors,
    screenshots: {
      default_canvas: DEFAULT_SCREENSHOT,
      indicator_drag: DRAG_SCREENSHOT,
      publication_preview: REPORT_SCREENSHOT,
    },
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
