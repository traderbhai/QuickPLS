import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { ensureDir, RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2400", "mockup-fidelity");
const OUTPUT = path.join(RESULTS, "v2400_mockup_fidelity_smoke.json");

const views = ["Home", "Data", "Model", "Setup", "Run", "Results", "Report", "Trust Center", "Settings"];

async function run() {
  const port = 57640;
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
    await page.goto(`${url}?native_shell=1&quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForSelector("[data-v237-native-prototype='true'][data-v240-mockup-fidelity='true']", { timeout: 10_000 });

    const snapshots = {};
    for (const label of views) {
      await page.locator(`.np-rail button:has-text("${label}")`).click();
      await page.waitForTimeout(150);
      const slug = label.toLowerCase().replace(/\s+/g, "-");
      await page.screenshot({ path: path.join(SCREEN_DIR, `${slug}.png`), fullPage: false });
      snapshots[slug] = await page.evaluate(() => ({
        noHorizontalOverflow: document.documentElement.scrollWidth <= window.innerWidth + 4,
        text: document.body.innerText.replace(/\s+/g, " ").slice(0, 2500),
      }));
    }

    await page.locator(".np-rail button:has-text('Model')").click();
    await page.waitForSelector("[data-v237-screen='model']", { timeout: 10_000 });
    const model = await page.evaluate(() => ({
      ribbon: Boolean(document.querySelector("[data-v240-ribbon='true']")),
      addLatent: document.body.innerText.includes("Add Latent"),
      connectPath: document.body.innerText.includes("Connect Path"),
      checkDiagram: document.body.innerText.includes("Check Diagram"),
      explorerTree: Boolean(document.querySelector("[data-v240-explorer-tree='true']")),
      inspectorTabs: Boolean(document.querySelector("[data-v240-inspector-tabs='true']")),
      bottomTabs: Boolean(document.querySelector("[data-v240-bottom-tabs='true']")),
      objectInspector: document.body.innerText.includes("Object Inspector"),
      diagramAdvisor: document.body.innerText.includes("Diagram Advisor"),
    }));

    await page.locator(".np-menu-slot button:has-text('Data')").click();
    await page.screenshot({ path: path.join(SCREEN_DIR, "menu-data-open.png"), fullPage: false });
    const menu = await page.evaluate(() => ({
      menuOpen: Boolean(document.querySelector(".np-menu-popover")),
      hasTransform: document.body.innerText.includes("Transform"),
      hasAddColumn: document.body.innerText.includes("Add Column"),
    }));

    return {
      model,
      menu,
      snapshots,
      consoleErrors: errors,
      shell: await page.evaluate(() => {
        const text = document.body.innerText;
        return {
          title: document.querySelector(".np-titlebar strong")?.textContent ?? "",
          hasFidelityMarker: Boolean(document.querySelector("[data-v240-mockup-fidelity='true']")),
          hasRibbon: Boolean(document.querySelector(".np-ribbon")),
          hasStatusbar: Boolean(document.querySelector(".np-statusbar")),
          noMojibake: !text.includes("RÃ") && !text.includes("Ã‚") && !text.includes("Â²"),
          noSmartPlsEquivalence: !/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(text),
        };
      }),
    };
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

await ensureDir(SCREEN_DIR);
const result = await run();
const checks = {
  fidelity_marker_present: result.shell.hasFidelityMarker,
  desktop_titlebar_current: result.shell.title.includes("QuickPLS 2.0"),
  ribbon_present: result.shell.hasRibbon && result.model.ribbon,
  ribbon_core_commands_present: result.model.addLatent && result.model.connectPath && result.model.checkDiagram,
  model_explorer_matches_mockup: result.model.explorerTree && result.model.objectInspector,
  model_inspector_tabs_present: result.model.inspectorTabs,
  bottom_output_tabs_present: result.model.bottomTabs && result.model.diagramAdvisor,
  menu_dropdown_available: result.menu.menuOpen && result.menu.hasTransform && result.menu.hasAddColumn,
  all_views_no_horizontal_overflow: Object.values(result.snapshots).every((snapshot) => snapshot.noHorizontalOverflow),
  no_console_errors: result.consoleErrors.length === 0,
  no_mojibake: result.shell.noMojibake,
  no_smartpls_equivalence_claim: result.shell.noSmartPlsEquivalence,
};
const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));
const payload = {
  passed: issues.length === 0,
  milestone: "v2_40_0_mockup_fidelity_native_shell_alignment",
  generatedAt: new Date().toISOString(),
  checks,
  issues,
  result,
};
await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.40 mockup-fidelity native shell screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
