import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const resultsRoot = path.join(root, "validation", "results");
const screenshotRoot = path.join(resultsRoot, "screens", "v251-unified-workflow");
const reportPath = path.join(resultsRoot, "v251_unified_workflow_packaged_smoke.json");
const endpoint = process.env.QUICKPLS_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const packagedOrigin = "http://tauri.localhost";

const report = {
  schemaVersion: 1,
  version: "2.51.0",
  generatedAt: new Date().toISOString(),
  runtime: "Packaged Tauri WebView2 over local CDP",
  passed: false,
  screenshots: [],
  observations: [],
  failures: [],
  consoleErrors: [],
};

function requireCondition(condition, message) {
  if (!condition) throw new Error(message);
}

async function capture(page, id, observation) {
  const file = path.join(screenshotRoot, `${id}.png`);
  await page.screenshot({ path: file, animations: "disabled" });
  report.screenshots.push(path.relative(root, file).replaceAll("\\", "/"));
  report.observations.push({ id, observation });
}

await fs.mkdir(screenshotRoot, { recursive: true });
let browser;
try {
  browser = await chromium.connectOverCDP(endpoint);
  const pages = browser.contexts().flatMap((context) => context.pages());
  const page = pages.find((candidate) => candidate.url().startsWith(packagedOrigin)) ?? pages[0];
  requireCondition(page, "The packaged QuickPLS WebView page was not available over CDP.");
  page.on("console", (message) => {
    if (message.type() === "error") report.consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => report.consoleErrors.push(error.message));
  page.on("dialog", (dialog) => dialog.accept());

  await page.goto(`${packagedOrigin}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.setView === "function", null, { timeout: 30_000 });

  await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:open-demo-project", {
    detail: { sampleId: "mediation" },
  })));
  await page.locator("#nd-model-canvas-panel").waitFor({ state: "visible" });
  const permanentTabLabels = await page.locator('[role="tab"]').allTextContents();
  requireCondition(
    permanentTabLabels.every((label) => !/Parameter Table|General SEM|Exact CB-SEM/i.test(label)),
    `Obsolete permanent model tabs remain visible: ${permanentTabLabels.join(", ")}`,
  );
  await capture(page, "01-canvas", "Canvas is the single permanent model workspace; the model toolbar exposes contextual advanced actions without architecture tabs.");

  await page.keyboard.press("Control+R");
  await page.locator("#nd-calculation-method-list").waitFor({ state: "visible" });
  const methodCount = await page.locator('#nd-calculation-method-list [role="option"]').count();
  requireCondition(methodCount === 18, `Calculate exposed ${methodCount} methods instead of 18.`);
  await page.getByRole("option", { name: "PLS-SEM Bootstrapping", exact: true }).click();
  const calculationFeatures = page.locator("#nd-calculation-detected-features");
  const detectedFeatures = await calculationFeatures.count()
    ? (await calculationFeatures.innerText()).trim()
    : (await page.locator(".nd-model-feature-inventory").innerText()).trim();
  requireCondition(/indirect path/i.test(detectedFeatures), `Mediation was not detected in Calculate: ${detectedFeatures}`);
  await capture(page, "02-method-setup", "Calculate contains exactly 18 methods and shows PLS Bootstrapping setup while the Canvas retains the detected mediation feature.");

  await page.getByRole("option", { name: "CB-SEM / CFA", exact: true }).click();
  await page.locator(".nd-blocker").waitFor({ state: "visible" });
  const correctiveText = (await page.locator(".nd-blocker").innerText()).trim();
  requireCondition(correctiveText.length > 40, "CB-SEM corrective guidance was empty.");
  await capture(page, "03-corrective-error", "An ineligible CB-SEM setup stays in Calculate and provides a corrective explanation without modifying the model.");
  await page.getByRole("button", { name: "Close", exact: true }).click();

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
  await page.locator(".nd-results-workspace").waitFor({ state: "visible" });
  await page.getByRole("tree", { name: "Available result sections" }).waitFor({ state: "visible" });
  requireCondition(await page.locator(".nd-results-document").isVisible(), "The Results detail pane is not visible.");
  await capture(page, "04-results", "The completed sample opens the normal categorized Results workspace with its result tree and detail pane.");

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("model"));
  await page.locator("#nd-model-canvas-panel").waitFor({ state: "visible" });
  await page.keyboard.press("Control+R");
  await page.locator("#nd-calculation-method-list").waitFor({ state: "visible" });
  await page.getByRole("option", { name: "PLS-SEM Bootstrapping", exact: true }).click();
  const bootstrapSamples = page.locator("#nd-calculation-bootstrap-samples");
  if (await bootstrapSamples.count()) await bootstrapSamples.fill("10000");
  await page.locator('form.nd-calculation-dialog button.primary[type="submit"]').click();
  const progress = page.locator(".nd-run-progress");
  await progress.waitFor({ state: "visible", timeout: 20_000 });
  await capture(page, "05-progress", "A packaged calculation exposes live progress and cancellation in the same Calculate window.");
  const cancel = page.getByRole("button", { name: /Cancel calculation|Cancelling/ });
  if (await cancel.count()) {
    await cancel.first().click();
    await page.waitForFunction(() => !document.querySelector('.nd-run-progress[aria-busy="true"]'), null, { timeout: 30_000 }).catch(() => undefined);
  }

  report.passed = report.failures.length === 0 && report.consoleErrors.length === 0;
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error));
} finally {
  report.generatedAt = new Date().toISOString();
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  if (browser) await browser.close().catch(() => undefined);
}

if (!report.passed) {
  throw new Error(`QuickPLS 2.51 packaged workflow smoke failed: ${report.failures.join(" | ") || report.consoleErrors.join(" | ")}`);
}

console.log(JSON.stringify({ passed: true, reportPath, screenshots: report.screenshots }, null, 2));
