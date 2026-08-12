import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { RESULTS, ensureDir, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const port = 57646;
const baseUrl = `http://127.0.0.1:${port}/`;
const screenshotDir = path.join(RESULTS, "screens", "v246-native-redesign");
const preview = startPreview(port);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
const evidence = { passed: false, screenshots: [], checks: {}, failures: [] };

async function capture(name) {
  const screenshot = path.join(screenshotDir, `${name}.png`);
  await page.screenshot({ path: screenshot, fullPage: false });
  evidence.screenshots.push(screenshot);
}

async function setSurface(surface) {
  const smokeView = surface === "launcher" ? "home" : surface;
  await page.evaluate((next) => window.__QUICKPLS_SMOKE__?.setView(next), smokeView);
  await page.waitForSelector(`.nd-app[data-surface="${surface}"]`);
}

try {
  await ensureDir(screenshotDir);
  await waitForPreview(baseUrl, preview.logs);
  await page.goto(`${baseUrl}?quickpls_smoke=1`, { waitUntil: "networkidle" });
  await page.waitForSelector('.nd-app[data-native-desktop-shell="true"]');

  await setSurface("launcher");
  await capture("01-launcher-1440x900");

  await setSurface("data");
  await capture("02-data-1440x900");

  await setSurface("model");
  await capture("03-model-1440x900");
  await page.locator('.nd-commandbar button', { hasText: "Calculate" }).first().click();
  await page.waitForSelector('.nd-dialog-calculation[role="dialog"]');
  await capture("04-calculation-dialog-1440x900");
  evidence.checks.fabricatedIterations = await page.getByText(/Iteration\s+\d+/i).count();
  await page.getByRole("button", { name: "Close dialog" }).click();

  await setSurface("results");
  await page.getByText("No completed calculation", { exact: true }).waitFor();
  await capture("05-empty-results-1440x900");

  await page.goto(`${baseUrl}?quickpls_smoke=completed`, { waitUntil: "networkidle" });
  await page.waitForSelector('.nd-app[data-native-desktop-shell="true"]');
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.addCompletedRun());
  await setSurface("results");
  await page.waitForSelector(".nd-result-tree");
  await page.waitForSelector(".nd-result-table");
  await capture("06-completed-results-1440x900");
  await page.locator('.nd-commandbar button', { hasText: "Export" }).first().click();
  await page.waitForSelector('.nd-dialog-export[role="dialog"]');
  await capture("07-export-dialog-1440x900");
  await page.getByRole("button", { name: "Close dialog" }).click();

  await page.setViewportSize({ width: 1024, height: 700 });
  await setSurface("model");
  await capture("08-model-compact-1024x700");

  evidence.checks.horizontalOverflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 2);
  evidence.checks.visibleHtmlTitlebars = await page.locator('.title-bar:visible, .np-titlebar:visible').count();
  evidence.checks.visibleLegacyRibbons = await page.locator('.ribbon:visible, .workflow-strip:visible, .np-rail:visible').count();
  evidence.checks.visibleButtonsInModel = await page.locator('button:visible').count();
  evidence.passed = !evidence.checks.horizontalOverflow && evidence.checks.visibleHtmlTitlebars === 0 && evidence.checks.visibleLegacyRibbons === 0 && evidence.checks.fabricatedIterations === 0;
  if (!evidence.passed) evidence.failures.push({ ...evidence.checks });
} catch (error) {
  evidence.failures.push({ type: "exception", message: String(error?.stack ?? error) });
} finally {
  await writeJson(path.join(RESULTS, "v246_native_redesign_visual_smoke.json"), evidence);
  await browser.close();
  stopPreview(preview.server, port);
}

if (!evidence.passed) {
  console.error(JSON.stringify(evidence.failures, null, 2));
  process.exit(1);
}

await fs.access(path.join(screenshotDir, "03-model-1440x900.png"));
console.log(JSON.stringify(evidence, null, 2));
