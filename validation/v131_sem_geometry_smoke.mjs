import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v131", "sem-geometry");
const OUTPUT = path.join(RESULTS, "v131_sem_geometry_smoke.json");
const PORT = 53131;
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

function segmentsCross(a, b) {
  if (a.source === b.source || a.source === b.target || a.target === b.source || a.target === b.target) return false;
  const o = (p, q, r) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
  return o(a.start, a.end, b.start) * o(a.start, a.end, b.end) < 0
    && o(b.start, b.end, a.start) * o(b.start, b.end, a.end) < 0;
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

  const fixtures = ["simple", "medium", "large", "mediation", "formative"];
  const evidence = {};
  for (const fixture of fixtures) {
    await page.evaluate((name) => window.__QUICKPLS_SMOKE__?.loadDiagramFixture(name), fixture);
    await page.waitForTimeout(350);
    const before = await collectMetrics(page);
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.arrangeSmartpls());
    await page.waitForTimeout(500);
    const screenshot = path.join(ARTIFACTS, `${fixture}.png`);
    await page.screenshot({ path: screenshot, fullPage: true });
    const after = await collectMetrics(page);
    evidence[fixture] = {
      before,
      after,
      screenshot,
      crossing_preserved_or_improved: after.path_crossing_count <= before.path_crossing_count,
      measurement_arrow_average_ok: after.average_measurement_path_length <= 190,
    };
  }

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadDiagramFixture("mediation"));
  await page.waitForTimeout(350);
  await page.getByRole("button", { name: /Arrange/i }).click();
  await page.getByRole("menuitem", { name: /Arrange like SmartPLS/i }).click();
  await page.waitForTimeout(350);
  await page.locator('select[aria-label="Diagram mode"]').selectOption("publication");
  await page.waitForTimeout(250);
  const publicationShot = path.join(ARTIFACTS, "publication-preview.png");
  await page.screenshot({ path: publicationShot, fullPage: true });

  const checklist = {
    simple_model_renders_academic_geometry: evidence.simple.after.latent_count >= 5 && evidence.simple.after.indicator_count >= 15,
    medium_model_no_latent_overlaps: evidence.medium.after.latent_overlap_count === 0,
    large_model_no_latent_overlaps: evidence.large.after.latent_overlap_count === 0,
    standard_fixtures_no_indicator_overlaps: ["simple", "medium", "mediation", "formative"].every((fixture) => evidence[fixture].after.indicator_overlap_count === 0),
    smartpls_layout_preserves_or_reduces_crossings: Object.values(evidence).every((item) => item.crossing_preserved_or_improved),
    measurement_lines_remain_compact: Object.values(evidence).every((item) => item.measurement_arrow_average_ok),
    labels_under_threshold: Object.values(evidence).every((item) => item.after.label_overlap_count <= Math.max(3, Math.floor(item.after.edge_label_count / 3))),
    arrowheads_and_edges_visible: Object.values(evidence).every((item) => item.after.edge_count > 0 && item.after.marker_count > 0),
    publication_preview_screenshot_written: Boolean(publicationShot),
  };

  const report = {
    schema_version: 1,
    target: "QuickPLS v1.3.1 SEM diagram geometry polish",
    passed: errors.length === 0 && Object.values(checklist).every(Boolean),
    checklist,
    errors,
    evidence: { fixtures: evidence, publication_preview: publicationShot },
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}

async function collectMetrics(page) {
  const data = await page.evaluate(() => {
    const rectFor = (selector) => [...document.querySelectorAll(selector)].map((node) => {
      const rect = node.getBoundingClientRect();
      return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height, x: rect.x, y: rect.y };
    });
    const latentRects = rectFor(".react-flow__node-latent");
    const indicatorRects = rectFor(".react-flow__node-indicator");
    const labelRects = rectFor(".sem-edge-label");
    const nodeCenters = new Map([...document.querySelectorAll(".react-flow__node")].map((node) => {
      const rect = node.getBoundingClientRect();
      return [node.getAttribute("data-id"), { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }];
    }));
    const structural = [...document.querySelectorAll(".react-flow__edge")].map((edge) => ({
      source: edge.getAttribute("data-source") ?? "",
      target: edge.getAttribute("data-target") ?? "",
      className: edge.getAttribute("class") ?? "",
    })).filter((edge) => edge.className.includes("structural-edge"));
    const segments = structural.map((edge) => ({
      source: edge.source,
      target: edge.target,
      start: nodeCenters.get(edge.source) ?? { x: 0, y: 0 },
      end: nodeCenters.get(edge.target) ?? { x: 0, y: 0 },
    }));
    const measurementLengths = [...document.querySelectorAll(".react-flow__edge")].filter((edge) => (edge.getAttribute("class") ?? "").includes("measurement-edge")).map((edge) => {
      const source = nodeCenters.get(edge.getAttribute("data-source") ?? "");
      const target = nodeCenters.get(edge.getAttribute("data-target") ?? "");
      return source && target ? Math.hypot(target.x - source.x, target.y - source.y) : 0;
    }).filter(Boolean);
    return {
      latent_count: latentRects.length,
      indicator_count: indicatorRects.length,
      edge_count: document.querySelectorAll(".react-flow__edge").length,
      edge_label_count: labelRects.length,
      marker_count: document.querySelectorAll("marker path, .react-flow__arrowhead").length,
      latent_rects: latentRects,
      indicator_rects: indicatorRects,
      label_rects: labelRects,
      segments,
      average_measurement_path_length: measurementLengths.length ? measurementLengths.reduce((sum, value) => sum + value, 0) / measurementLengths.length : 0,
    };
  });
  let crossingCount = 0;
  for (let i = 0; i < data.segments.length; i += 1) {
    for (let j = i + 1; j < data.segments.length; j += 1) {
      if (segmentsCross(data.segments[i], data.segments[j])) crossingCount += 1;
    }
  }
  return {
    ...data,
    latent_overlap_count: overlaps(data.latent_rects),
    indicator_overlap_count: overlaps(data.indicator_rects),
    label_overlap_count: overlaps(data.label_rects),
    path_crossing_count: crossingCount,
  };
}
