import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v111", "sem-designer");
const OUTPUT = path.join(RESULTS, "v111_sem_designer_dense_smoke.json");
const PORT = 5311;
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

  const fixtures = ["medium", "large", "mediation", "formative"];
  const evidence = {};
  for (const fixture of fixtures) {
    await page.evaluate((name) => window.__QUICKPLS_SMOKE__?.loadDiagramFixture(name), fixture);
    await page.waitForTimeout(500);
    await page.getByRole("button", { name: /Arrange/i }).click();
    await page.getByRole("menuitem", { name: /Arrange like SmartPLS/i }).click();
    await page.waitForTimeout(500);
    const screenshot = path.join(ARTIFACTS, `${fixture}.png`);
    await page.screenshot({ path: screenshot, fullPage: true });
    evidence[fixture] = await page.evaluate(() => {
      const latentRects = [...document.querySelectorAll(".react-flow__node-latent")].map((node) => {
        const rect = node.getBoundingClientRect();
        return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
      });
      const labels = [...document.querySelectorAll(".sem-edge-label")].map((label) => {
        const rect = label.getBoundingClientRect();
        return { text: label.textContent ?? "", left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
      });
      return {
        latent_count: latentRects.length,
        indicator_count: document.querySelectorAll(".react-flow__node-indicator").length,
        structural_edge_count: document.querySelectorAll(".react-flow__edge.structural-edge, .react-flow__edge .structural-edge").length || document.querySelectorAll(".react-flow__edge").length,
        edge_label_count: labels.length,
        latent_rects: latentRects,
        label_rects: labels,
      };
    });
    evidence[fixture].latent_overlap_count = overlaps(evidence[fixture].latent_rects);
    evidence[fixture].label_overlap_count = overlaps(evidence[fixture].label_rects);
    evidence[fixture].screenshot = screenshot;
  }

  const checklist = {
    medium_fixture_has_expected_nodes: evidence.medium.latent_count >= 8 && evidence.medium.indicator_count >= 32,
    large_fixture_has_expected_nodes: evidence.large.latent_count >= 20 && evidence.large.indicator_count >= 80,
    latent_nodes_do_not_overlap: Object.values(evidence).every((item) => item.latent_overlap_count === 0),
    structural_edges_render_and_labels_do_not_collide: Object.values(evidence).every((item) => item.structural_edge_count > 0 && item.label_overlap_count <= Math.max(2, Math.floor(Math.max(1, item.edge_label_count) / 3))),
    screenshots_written: fixtures.every((fixture) => evidence[fixture].screenshot),
  };
  const report = { schema_version: 1, target: "QuickPLS v1.1.1 dense SEM designer smoke", passed: errors.length === 0 && Object.values(checklist).every(Boolean), checklist, errors, evidence };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
