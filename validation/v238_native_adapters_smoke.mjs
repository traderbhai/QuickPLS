import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { ensureDir, RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2380", "native-adapters");
const OUTPUT = path.join(RESULTS, "v2380_native_frontend_backend_adapters_smoke.json");
const views = ["home", "data", "model", "setup", "run", "results", "report", "trust"];

async function run() {
  const port = 57638;
  const url = `http://127.0.0.1:${port}/`;
  const { server, logs } = startPreview(port);
  let browser;
  try {
    await waitForPreview(url, logs);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${url}?native_prototype=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForSelector("[data-v237-native-prototype='true'][data-v238-adapter]", { timeout: 10_000 });

    const snapshots = {};
    for (const view of views) {
      const label = view === "trust" ? "Trust Center" : view.charAt(0).toUpperCase() + view.slice(1);
      await page.locator(`.np-rail button:has-text("${label}")`).click();
      await page.waitForSelector(`[data-v237-screen='${view}']`, { timeout: 10_000 });
      await page.screenshot({ path: path.join(SCREEN_DIR, `${view}.png`), fullPage: false });
      snapshots[view] = await page.evaluate((screen) => {
        const root = document.querySelector(`[data-v237-screen='${screen}']`);
        const rect = document.documentElement.getBoundingClientRect();
        return {
          present: Boolean(root),
          text: root?.textContent?.replace(/\s+/g, " ").slice(0, 1200) ?? "",
          noHorizontalOverflow: document.documentElement.scrollWidth <= Math.ceil(rect.width) + 4,
        };
      }, view);
    }

    const adapter = await page.evaluate(() => {
      const root = document.querySelector("[data-v237-native-prototype='true']");
      const bodyText = document.body.innerText;
      return {
        source: root?.getAttribute("data-v238-adapter"),
        titlebarText: document.querySelector(".np-titlebar")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
        statusbarText: document.querySelector(".np-statusbar")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
        dataMentionsStoreDataset: /corporate_reputation\.csv|Corporate Reputation/i.test(bodyText),
        modelMentionsStoreConstructs: true,
        methodSelectorConservative: document.querySelector(".np-commandbar select")?.textContent?.includes("More methods in Setup") ?? false,
        noMojibake: !bodyText.includes("RÃ") && !bodyText.includes("Â²"),
      };
    });

    return { snapshots, adapter, consoleErrors: errors };
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

await ensureDir(SCREEN_DIR);
const result = await run();
const checks = {
  all_views_render: views.every((view) => result.snapshots[view]?.present),
  adapter_attribute_present: ["store", "fallback"].includes(result.adapter.source),
  store_adapter_active: result.adapter.source === "store",
  store_dataset_visible: result.adapter.dataMentionsStoreDataset,
  store_constructs_visible: /Competence|Likeability|Customer Satisfaction|Customer Loyalty|COMP|LIKE|CUSA|CUSL/i.test(result.snapshots.model?.text ?? ""),
  statusbar_uses_project_data: /Corporate Reputation|Customer Loyalty/.test(result.adapter.statusbarText),
  method_selector_conservative: result.adapter.methodSelectorConservative,
  no_console_errors: result.consoleErrors.length === 0,
  no_horizontal_overflow: Object.values(result.snapshots).every((snapshot) => snapshot.noHorizontalOverflow),
  no_mojibake: result.adapter.noMojibake,
};
const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));
const payload = {
  passed: issues.length === 0,
  milestone: "v2_38_0_native_frontend_backend_adapters",
  generatedAt: new Date().toISOString(),
  checks,
  issues,
  result,
};
await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.38 native frontend backend adapter screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
