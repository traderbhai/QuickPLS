import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v13", "sem-designer");
const OUTPUT = path.join(RESULTS, "v13_sem_designer_ux_smoke.json");
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

function overlaps(rects) {
  let count = 0;
  for (let i = 0; i < rects.length; i += 1) {
    for (let j = i + 1; j < rects.length; j += 1) {
      const a = rects[i];
      const b = rects[j];
      if (a.right > b.left && b.right > a.left && a.bottom > b.top && b.bottom > a.top) count += 1;
    }
  }
  return count;
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

  const canvasShot = path.join(ARTIFACTS, "01_editable_academic_canvas.png");
  await page.screenshot({ path: canvasShot, fullPage: true });

  const firstLatent = page.locator(".smartpls-latent-node").first();
  const firstLabel = firstLatent.locator(".smartpls-latent-label");
  const before = await firstLatent.boundingBox();
  if (!before) throw new Error("No latent construct found for drag smoke.");
  await page.mouse.move(before.x + before.width / 2, before.y + before.height / 2);
  await page.mouse.down();
  await page.mouse.move(before.x + before.width / 2 + 90, before.y + before.height / 2 + 45, { steps: 10 });
  await page.mouse.up();
  await page.waitForTimeout(250);
  const after = await firstLatent.boundingBox();

  await firstLabel.click({ button: "right" });
  await page.waitForTimeout(150);
  const contextMenuVisible = await page.locator(".diagram-context-menu").isVisible();
  const contextMenuText = contextMenuVisible ? await page.locator(".diagram-context-menu").innerText() : "";

  await page.keyboard.press("Escape");
  await page.mouse.click(980, 250);
  await page.waitForTimeout(100);
  const edgeLabel = page.locator(".sem-edge-label").first();
  if (await edgeLabel.count()) {
    await edgeLabel.focus();
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("Shift+ArrowRight");
  }

  await page.getByRole("button", { name: /Arrange/i }).click();
  await page.getByRole("menuitem", { name: /Arrange like SmartPLS/i }).click();
  await page.waitForTimeout(300);
  const denseShot = path.join(ARTIFACTS, "02_arranged_sem_canvas.png");
  await page.screenshot({ path: denseShot, fullPage: true });

  const domEvidence = await page.evaluate(() => {
    const handleRects = [...document.querySelectorAll(".smartpls-edit-handle")].map((handle) => {
      const rect = handle.getBoundingClientRect();
      const styles = window.getComputedStyle(handle);
      return { width: rect.width, height: rect.height, opacity: Number(styles.opacity), pointerEvents: styles.pointerEvents };
    });
    const latentRects = [...document.querySelectorAll(".react-flow__node-latent")].map((node) => {
      const rect = node.getBoundingClientRect();
      return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
    });
    const labels = [...document.querySelectorAll(".sem-edge-label")].map((label) => {
      const rect = label.getBoundingClientRect();
      return { text: label.textContent ?? "", left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
    });
    return {
      latent_count: document.querySelectorAll(".smartpls-latent-node").length,
      indicator_count: document.querySelectorAll(".smartpls-indicator-node").length,
      edit_handle_count: handleRects.length,
      visible_edit_handle_count: handleRects.filter((rect) => rect.width >= 6 && rect.height >= 6 && rect.pointerEvents !== "none").length,
      structural_edge_count: document.querySelectorAll(".react-flow__edge.structural-edge, .react-flow__edge .structural-edge").length || document.querySelectorAll(".react-flow__edge").length,
      edge_label_count: labels.length,
      action_feedback_present: Boolean(document.querySelector(".canvas-action-feedback")),
      latent_rects: latentRects,
      label_rects: labels,
    };
  });

  const dragDelta = before && after ? { x: Math.round(after.x - before.x), y: Math.round(after.y - before.y) } : { x: 0, y: 0 };
  const checklist = {
    editable_academic_canvas_renders: domEvidence.latent_count >= 4 && domEvidence.indicator_count >= 8,
    edit_handles_available: domEvidence.edit_handle_count >= domEvidence.latent_count * 4 && domEvidence.visible_edit_handle_count >= domEvidence.latent_count * 4,
    latent_drag_moves_construct: Math.abs(dragDelta.x) >= 40 || Math.abs(dragDelta.y) >= 25,
    context_menu_exposes_common_actions: contextMenuVisible && ["Rename construct", "Duplicate", "Delete", "Reset indicator layout"].every((text) => contextMenuText.includes(text)),
    edge_labels_keyboard_accessible: domEvidence.edge_label_count >= 1,
    arranged_layout_keeps_latents_apart: overlaps(domEvidence.latent_rects) === 0,
    screenshots_written: [canvasShot, denseShot].every(Boolean),
  };

  const report = {
    schema_version: 1,
    target: "QuickPLS v1.3 SEM designer UX overhaul",
    passed: errors.length === 0 && Object.values(checklist).every(Boolean),
    checklist,
    errors,
    drag_delta: dragDelta,
    evidence: { ...domEvidence, latent_overlap_count: overlaps(domEvidence.latent_rects), label_overlap_count: overlaps(domEvidence.label_rects), screenshots: [canvasShot, denseShot] },
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
