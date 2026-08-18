import { performance } from "node:perf_hooks";
import { chromium } from "playwright";
import {
  RESULTS,
  startPreview,
  stopPreview,
  waitForPreview,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";
import path from "node:path";

const port = 57_648;
const url = `http://127.0.0.1:${port}/?quickpls_smoke=1`;
const resultPath = path.join(RESULTS, "v247_large_model_focused_acceptance.json");
const allProfiles = [
  { id: "applied_diagram", token: "large", constructs: 20, indicators: 80, openMs: 2_000, editP95Ms: 100, minimumFps: 45 },
  { id: "stress_diagram", token: "large-stress", constructs: 100, indicators: 300, openMs: 10_000, editP95Ms: 250, minimumFps: 30 },
];
const requestedProfile = process.argv.find((argument) => argument.startsWith("--profile="))?.split("=")[1];
const profiles = requestedProfile
  ? allProfiles.filter((profile) => profile.id === requestedProfile || profile.token === requestedProfile || profile.id.startsWith(`${requestedProfile}_`))
  : allProfiles;
if (profiles.length === 0) throw new Error(`Unknown focused profile: ${requestedProfile}`);

const percentile = (values, probability) => {
  const ordered = [...values].sort((left, right) => left - right);
  const index = (ordered.length - 1) * probability;
  const lower = Math.floor(index);
  const fraction = index - lower;
  return ordered[lower] + (ordered[Math.min(lower + 1, ordered.length - 1)] - ordered[lower]) * fraction;
};

async function counts(page) {
  return page.evaluate(() => window.__QUICKPLS_SMOKE__?.modelCounts?.() ?? ({ constructs: 0, indicators: 0 }));
}

async function loadFixture(page, profile) {
  const started = performance.now();
  await page.evaluate(async (token) => {
    window.__QUICKPLS_SMOKE__?.loadEmptyProject?.();
    await window.__QUICKPLS_SMOKE__?.loadDiagramFixture?.(token);
  }, profile.token);
  await page.waitForFunction(({ constructs, indicators }) => {
    const counts = window.__QUICKPLS_SMOKE__?.modelCounts?.();
    return counts != null && counts.constructs >= constructs && counts.indicators >= indicators;
  }, profile, { timeout: profile.openMs });
  return Math.round((performance.now() - started) * 10) / 10;
}

async function sample(page, profile) {
  const openMs = await loadFixture(page, profile);
  const observed = await counts(page);
  const targetConstructId = "construct-1";
  await page.evaluate((id) => {
    window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id } }));
  }, targetConstructId);
  await page.waitForTimeout(320);
  const firstNode = page.locator(`.react-flow__node[data-id="${targetConstructId}"]`).first();
  await firstNode.waitFor({ state: "visible", timeout: 2_000 });
  const firstConstruct = firstNode.locator(".smartpls-latent-node");

  const selectionStarted = performance.now();
  await firstConstruct.click({ timeout: 2_000 }).catch(() => null);
  await page.waitForFunction((id) => document.querySelector(`.react-flow__node[data-id="${id}"].selected, .react-flow__node[data-id="${id}"] .smartpls-latent-node.selected`) != null, targetConstructId, { timeout: 2_000 }).catch(() => null);
  const selectionMs = Math.round((performance.now() - selectionStarted) * 10) / 10;
  const selectionChanged = await firstNode.evaluate((node) => node.classList.contains("selected") || node.querySelector(".smartpls-latent-node.selected") != null).catch(() => false);

  const beforeDrag = await firstNode.boundingBox().catch(() => null);
  let editResponseMs = null;
  let gestureCommitMs = null;
  if (beforeDrag) {
    await page.mouse.move(beforeDrag.x + beforeDrag.width / 2, beforeDrag.y + beforeDrag.height / 2);
    await page.mouse.down();
    const dragStarted = performance.now();
    await page.evaluate(({ id, x, y }) => {
      const node = document.querySelector(`.react-flow__node[data-id="${id}"]`);
      window.__QUICKPLS_DRAG_RESPONSE__ = new Promise((resolve) => {
        if (!(node instanceof HTMLElement)) {
          resolve(null);
          return;
        }
        let started = null;
        let timeoutId = 0;
        let finished = false;
        const finish = (value) => {
          if (finished) return;
          finished = true;
          window.removeEventListener("pointermove", onPointerMove, true);
          window.clearTimeout(timeoutId);
          resolve(value);
        };
        const checkMoved = (frameTime) => {
          const next = node.getBoundingClientRect();
          if (Math.hypot(next.x - x, next.y - y) >= 4) {
            // A second frame starts after the first moved frame has been painted.
            window.requestAnimationFrame((paintedFrameTime) => {
              finish(Math.round((paintedFrameTime - started) * 10) / 10);
            });
          } else if (frameTime - started >= 1_000) {
            finish(null);
          } else {
            window.requestAnimationFrame(checkMoved);
          }
        };
        const onPointerMove = () => {
          if (started !== null) return;
          started = window.performance.now();
          window.requestAnimationFrame(checkMoved);
        };
        window.addEventListener("pointermove", onPointerMove, { capture: true, once: true });
        timeoutId = window.setTimeout(() => finish(null), 1_500);
      });
    }, { id: targetConstructId, x: beforeDrag.x, y: beforeDrag.y });
    await page.mouse.move(beforeDrag.x + beforeDrag.width / 2 + 48, beforeDrag.y + beforeDrag.height / 2 + 32, { steps: 2 });
    editResponseMs = await page.evaluate(() => window.__QUICKPLS_DRAG_RESPONSE__ ?? null);
    await page.mouse.up();
    await page.waitForTimeout(80);
    gestureCommitMs = Math.round((performance.now() - dragStarted) * 10) / 10;
  }
  const afterDrag = await firstNode.boundingBox().catch(() => null);
  const dragChanged = Boolean(beforeDrag && afterDrag && Math.hypot(afterDrag.x - beforeDrag.x, afterDrag.y - beforeDrag.y) >= 4);

  const viewport = page.locator(".react-flow__viewport");
  const beforePan = await viewport.evaluate((node) => getComputedStyle(node).transform).catch(() => "");
  const panButton = page.locator(".nd-commandbar button").filter({ hasText: /^Pan$/ }).first();
  await panButton.click({ timeout: 1_000 }).catch(() => null);
  await page.waitForFunction(() => document.querySelector('.nd-commandbar button[aria-pressed="true"]')?.textContent?.trim() === "Pan", undefined, { timeout: 1_000 }).catch(() => null);
  const safePoint = await page.evaluate(() => {
    const pane = document.querySelector(".react-flow__pane");
    if (!(pane instanceof HTMLElement)) return null;
    const rect = pane.getBoundingClientRect();
    for (const [xRatio, yRatio] of [[0.9, 0.9], [0.1, 0.9], [0.9, 0.2], [0.5, 0.9]]) {
      const x = rect.left + rect.width * xRatio;
      const y = rect.top + rect.height * yRatio;
      const hit = document.elementFromPoint(x, y);
      if (hit && !hit.closest(".react-flow__node, .react-flow__edge, .canvas-toolbar, .react-flow__controls")) return { x, y };
    }
    return null;
  });
  if (safePoint) {
    await page.mouse.move(safePoint.x, safePoint.y);
    await page.mouse.down();
    await page.mouse.move(safePoint.x + 54, safePoint.y + 36, { steps: 8 });
    await page.mouse.up();
    await page.waitForTimeout(80);
  }
  const afterPan = await viewport.evaluate((node) => getComputedStyle(node).transform).catch(() => beforePan);
  const panChanged = afterPan !== beforePan;

  const beforeZoom = await viewport.evaluate((node) => getComputedStyle(node).transform).catch(() => "");
  await page.locator(".react-flow__controls-zoomin").first().click().catch(() => null);
  await page.waitForTimeout(280);
  const afterZoom = await viewport.evaluate((node) => getComputedStyle(node).transform).catch(() => beforeZoom);
  const zoomChanged = afterZoom !== beforeZoom;

  const preflightStarted = performance.now();
  const preflight = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.modelPreflight?.() ?? null).catch(() => null);
  const preflightMs = Math.round((performance.now() - preflightStarted) * 10) / 10;

  const fpsPromise = page.evaluate(() => new Promise((resolve) => {
    let frames = 0;
    const started = performance.now();
    const tick = (now) => {
      frames += 1;
      if (now - started >= 500) resolve(frames * 2);
      else requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }));
  await page.mouse.wheel(0, -240);
  const panZoomFps = await fpsPromise;

  const fixtureReloadMs = await loadFixture(page, profile);
  const reopened = await counts(page);
  return {
    openMs, selectionMs, editResponseMs, gestureCommitMs, preflightMs, panZoomFps,
    fixtureReloadMs, observed, reopened,
    preflight,
    interactions: {
      selectionChanged,
      dragChanged, panChanged, zoomChanged,
      preflightPresent: Boolean(preflight && [preflight.ready, preflight.blockers, preflight.warnings].every(Number.isInteger)
        && preflight.ready + preflight.blockers + preflight.warnings > 0),
      fixtureReloaded: reopened.constructs === profile.constructs && reopened.indicators === profile.indicators,
    },
  };
}

const preview = startPreview(port);
let browser;
const output = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  harness: "Chromium production preview; one warm-up plus five measured runs",
  editResponseScope: "Browser pointermove event to the first fully painted frame with a moved node; Playwright protocol time is excluded and total gesture time remains recorded separately.",
  archiveReopenMeasured: false,
  reopenScope: "Deterministic fixture reload only; this is not saved-project archive reopen evidence.",
  profiles: [],
  passed: false,
};

try {
  await waitForPreview(url, preview.logs);
  browser = await chromium.launch({ headless: true });
  for (const profile of profiles) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture === "function");
    const warmup = await sample(page, profile);
    const samples = [];
    for (let index = 0; index < 5; index += 1) samples.push(await sample(page, profile));
    const aggregates = {
      openP95Ms: percentile(samples.map((item) => item.openMs), 0.95),
      editResponseP95Ms: percentile(samples.map((item) => Number.isFinite(item.editResponseMs) ? item.editResponseMs : Number.POSITIVE_INFINITY), 0.95),
      minimumPanZoomFps: Math.min(...samples.map((item) => item.panZoomFps)),
    };
    const interactionsPassed = samples.every((item) => Object.values(item.interactions).every(Boolean));
    const budgetsPassed = aggregates.openP95Ms <= profile.openMs
      && aggregates.editResponseP95Ms <= profile.editP95Ms
      && aggregates.minimumPanZoomFps >= profile.minimumFps;
    output.profiles.push({ profile, warmup, samples, aggregates, interactionsPassed, budgetsPassed, passed: interactionsPassed && budgetsPassed });
    await page.close();
  }
  output.passed = output.profiles.every((profile) => profile.passed);
} catch (error) {
  output.error = String(error?.stack ?? error);
} finally {
  if (browser) await browser.close();
  stopPreview(preview.server, port);
  await writeJson(resultPath, output);
}

console.log(JSON.stringify(output, null, 2));
if (!output.passed) process.exit(1);
