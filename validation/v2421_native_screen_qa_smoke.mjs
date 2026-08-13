import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { RESULTS, ensureDir, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const port = 57643;
const baseUrl = `http://127.0.0.1:${port}/`;
const screenshotDir = path.join(RESULTS, "screens", "v2421", "native-qa");

const screens = [
  { id: "home", rail: "Home", selector: '[data-v237-screen="home"]', expected: ["Project Manager", "Recent Projects", "Project Details / Getting Started"] },
  { id: "data", rail: "Data", selector: '[data-native-functional-workspace="data"]', expected: ["DATA WORKSPACE", "Data View", "Data preview", "Column profile"] },
  { id: "model", rail: "Model", selector: '[data-native-functional-workspace="model"]', expected: ["MODEL WORKSPACE", "What can I do with this model?", "Model issues", "Essentials"] },
  { id: "setup", rail: "Setup", selector: '[data-native-functional-workspace="setup"]', expected: ["Calculation Setup", "Recommended methods appear first", "Data, model, method, and scope checks"] },
  { id: "run", rail: "Run", selector: '[data-native-functional-workspace="run"]', expected: ["Run analysis", "Procedure", "Run settings"] },
  { id: "results", rail: "Results", selector: '[data-native-functional-workspace="results"]', expected: ["Results", "Result workbook"] },
  { id: "report", rail: "Report", selector: '[data-native-functional-workspace="report"]', expected: ["Publication report", "Report package"] },
  { id: "trust", rail: "Trust Center", selector: '[data-native-functional-workspace="trust"]', expected: ["Trust Center", "Evidence documents"] },
  { id: "settings", rail: "Settings", selector: '[data-native-functional-workspace="settings"]', expected: ["Settings", "Numerical boundary", "Local preferences"] },
];

const dialogs = [
  { id: "import_data", screen: "data", rail: "Data", button: "Import Data", expected: ["Source", "Options", "Preview", "Validation summary"] },
  { id: "calculation_setup", screen: "setup", rail: "Setup", button: "Setup", expected: ["Algorithm settings", "Random seed", "Output preview"] },
];

function missingTerms(text, expected) {
  return expected.filter((term) => !text.includes(term));
}

async function clickRail(page, label) {
  await page.locator(".np-rail button", { hasText: label }).first().click();
}

async function hasHorizontalOverflow(page) {
  return page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 2);
}

const preview = startPreview(port);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
const evidence = {
  passed: false,
  port,
  screenshots: [],
  screens: [],
  dialogs: [],
  failures: [],
};

try {
  await ensureDir(screenshotDir);
  await waitForPreview(baseUrl, preview.logs);
  await page.goto(`${baseUrl}?quickpls_smoke=1`, { waitUntil: "networkidle" });
  await page.waitForSelector(".np-shell[data-v241-mockup-parity='true']", { timeout: 20000 });

  for (const screen of screens) {
    await clickRail(page, screen.rail);
    await page.waitForSelector(screen.selector, { timeout: 10000 });
    const screenshot = path.join(screenshotDir, `${screen.id}.png`);
    await page.screenshot({ path: screenshot, fullPage: false });
    const text = await page.locator(screen.selector).first().innerText();
    const missing = missingTerms(text, screen.expected);
    const overflow = await hasHorizontalOverflow(page);
    const result = { ...screen, screenshot, missing, overflow, passed: missing.length === 0 && !overflow };
    evidence.screens.push(result);
    evidence.screenshots.push(screenshot);
    if (!result.passed) {
      evidence.failures.push({ type: "screen", id: screen.id, missing, overflow });
    }
  }

  for (const dialog of dialogs) {
    await clickRail(page, dialog.rail);
    const screen = screens.find((item) => item.id === dialog.screen);
    await page.waitForSelector(screen?.selector ?? `[data-v237-screen="${dialog.screen}"]`, { timeout: 10000 });
    await page.locator(".np-commandbar button", { hasText: dialog.button }).first().click();
    await page.waitForSelector(`[data-v237-dialog="${dialog.id}"]`, { timeout: 10000 });
    const screenshot = path.join(screenshotDir, `dialog-${dialog.id}.png`);
    await page.screenshot({ path: screenshot, fullPage: false });
    const text = await page.locator(`[data-v237-dialog="${dialog.id}"]`).innerText();
    const missing = missingTerms(text, dialog.expected);
    const overflow = await hasHorizontalOverflow(page);
    const result = { ...dialog, screenshot, missing, overflow, passed: missing.length === 0 && !overflow };
    evidence.dialogs.push(result);
    evidence.screenshots.push(screenshot);
    if (!result.passed) {
      evidence.failures.push({ type: "dialog", id: dialog.id, missing, overflow });
    }
    await page.locator(`[data-v237-dialog="${dialog.id}"] button[aria-label="Close dialog"]`).click();
    await page.waitForSelector(`[data-v237-dialog="${dialog.id}"]`, { state: "detached", timeout: 10000 });
  }

  evidence.passed = evidence.failures.length === 0;
} catch (error) {
  evidence.failures.push({ type: "exception", message: String(error?.message ?? error) });
} finally {
  await writeJson(path.join(RESULTS, "v2421_native_screen_qa_smoke.json"), evidence);
  await browser.close();
  stopPreview(preview.server, port);
}

if (!evidence.passed) {
  console.error(JSON.stringify(evidence.failures, null, 2));
  process.exit(1);
}
