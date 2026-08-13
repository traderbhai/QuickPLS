import path from "node:path";
import { chromium } from "playwright";
import { RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const port = 57644;
const baseUrl = `http://127.0.0.1:${port}/`;

const evidence = {
  passed: false,
  checks: [],
  failures: [],
};

function record(name, passed, details = {}) {
  const check = { name, passed, ...details };
  evidence.checks.push(check);
  if (!passed) evidence.failures.push(check);
}

async function clickRail(page, label, id) {
  await page.locator(".np-rail button", { hasText: label }).first().click();
  const selector = id === "home" ? '[data-v237-screen="home"]' : `[data-native-functional-workspace="${id}"]`;
  await page.waitForSelector(selector, { timeout: 10000 });
  record(`rail opens ${label}`, true, { screen: id });
}

async function openDialogFrom(page, label, dialogId) {
  await page.locator(".np-commandbar button", { hasText: label }).first().click();
  await page.waitForSelector(`[data-v237-dialog="${dialogId}"]`, { timeout: 10000 });
}

const preview = startPreview(port);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });

try {
  await waitForPreview(baseUrl, preview.logs);
  await page.goto(`${baseUrl}?quickpls_smoke=1`, { waitUntil: "networkidle" });
  await page.waitForSelector(".np-shell[data-v241-mockup-parity='true']", { timeout: 20000 });

  const mode = await page.locator(".np-shell").getAttribute("data-v239-shell-mode");
  const adapter = await page.locator(".np-shell").getAttribute("data-v238-adapter");
  record("default route mounts native shell", mode === "production-candidate", { mode });
  record("native shell uses backend adapter data", adapter !== "static", { adapter });

  const menus = ["File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Tools", "Window", "Help"];
  for (const menu of menus) {
    const button = page.locator(".np-menu-slot > button", { hasText: menu }).first();
    await button.click();
    const openCount = await page.locator(".np-menu-popover").count();
    await button.click();
    const closedCount = await page.locator(".np-menu-popover").count();
    record(`menu toggles ${menu}`, openCount === 1 && closedCount === 0, { openCount, closedCount });
  }

  for (const [label, id] of [["Home", "home"], ["Data", "data"], ["Model", "model"], ["Setup", "setup"], ["Run", "run"], ["Results", "results"], ["Report", "report"], ["Trust Center", "trust"], ["Settings", "settings"]]) {
    await clickRail(page, label, id);
  }

  await clickRail(page, "Data", "data");
  await openDialogFrom(page, "Import Data", "import_data");
  await page.locator(`[data-v237-dialog="import_data"] button[aria-label="Close dialog"]`).click();
  await page.waitForSelector(`[data-v237-dialog="import_data"]`, { state: "detached", timeout: 10000 });
  record("import dialog closes by close button", true);

  await openDialogFrom(page, "Import Data", "import_data");
  await page.locator(`[data-v237-dialog="import_data"] button`, { hasText: "Cancel" }).click();
  await page.waitForSelector(`[data-v237-dialog="import_data"]`, { state: "detached", timeout: 10000 });
  record("import dialog closes by Cancel", true);

  await clickRail(page, "Run", "run");
  await openDialogFrom(page, "Setup", "calculation_setup");
  await page.keyboard.press("Tab");
  const focusInsideDialog = await page.evaluate(() => Boolean(document.activeElement?.closest?.("[data-v237-dialog='calculation_setup']")));
  record("dialog focus remains inside calculation dialog after Tab", focusInsideDialog);
  await page.keyboard.press("Escape");
  await page.waitForSelector(`[data-v237-dialog="calculation_setup"]`, { state: "detached", timeout: 10000 });
  record("calculation dialog closes by Escape", true);

  await clickRail(page, "Model", "model");
  const modelSelectors = {
    explorer: await page.locator('[data-native-functional-workspace="model"] .explorer').count(),
    canvas: await page.locator('[data-native-functional-workspace="model"] .react-flow').count(),
    inspector: await page.locator('[data-native-functional-workspace="model"] .inspector').count(),
    issues: await page.locator('[data-native-functional-workspace="model"] .model-issues-pane, [data-native-functional-workspace="model"] .model-v225-bottom-pane').count(),
  };
  record("SEM designer remains integrated, not static-only", Object.values(modelSelectors).every((count) => count > 0), modelSelectors);

  await clickRail(page, "Run", "run");
  record("Run reads existing settings summary", (await page.locator('[data-native-functional-workspace="run"]', { hasText: "Run analysis" }).count()) > 0);
  await clickRail(page, "Results", "results");
  record("Results reads existing run/workbook data", (await page.locator('[data-native-functional-workspace="results"]', { hasText: "Result workbook" }).count()) > 0);
  await clickRail(page, "Report", "report");
  record("Report reads existing run/report data", (await page.locator('[data-native-functional-workspace="report"]', { hasText: "Publication report" }).count()) > 0);

  evidence.passed = evidence.failures.length === 0;
} catch (error) {
  record("interaction smoke exception", false, { message: String(error?.message ?? error) });
} finally {
  await writeJson(path.join(RESULTS, "v2421_native_interaction_wiring_smoke.json"), evidence);
  await browser.close();
  stopPreview(preview.server, port);
}

if (!evidence.passed) {
  console.error(JSON.stringify(evidence.failures, null, 2));
  process.exit(1);
}
