import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { ensureDir, RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2370", "native-frontend-prototype");
const OUTPUT = path.join(RESULTS, "v2370_native_frontend_prototype_smoke.json");
const views = ["home", "data", "model", "setup", "run", "results", "report", "trust", "settings"];
const dialogs = ["new_project", "sample_gallery", "import_data", "calculation_setup", "method_scope", "export_options", "help_shortcuts"];
const viewports = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "desktop-1280", width: 1280, height: 800 },
];

async function runViewport(viewport, index) {
  const port = 57600 + index;
  const url = `http://127.0.0.1:${port}/`;
  const { server, logs } = startPreview(port);
  let browser;
  try {
    await waitForPreview(url, logs);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport, deviceScaleFactor: 1 });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${url}?native_prototype=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForSelector("[data-v237-native-prototype='true']", { timeout: 10_000 });

    const snapshots = {};
    for (const view of views) {
      await page.locator(`.np-rail button:has-text("${view === "trust" ? "Trust Center" : view.charAt(0).toUpperCase() + view.slice(1)}")`).click();
      await page.waitForSelector(`[data-v237-screen='${view}']`, { timeout: 10_000 });
      await page.screenshot({ path: path.join(SCREEN_DIR, `${viewport.name}-${view}.png`), fullPage: false });
      snapshots[view] = await page.evaluate((screen) => {
        const root = document.querySelector(`[data-v237-screen='${screen}']`);
        const rect = document.documentElement.getBoundingClientRect();
        return {
          present: Boolean(root),
          text: root?.textContent?.replace(/\s+/g, " ").slice(0, 800) ?? "",
          noHorizontalOverflow: document.documentElement.scrollWidth <= Math.ceil(rect.width) + 4,
        };
      }, view);
    }

    await page.locator(".np-rail button:has-text('Home')").click();
    const dialogSnapshots = {};
    for (const dialog of dialogs) {
      if (dialog === "import_data") {
        await page.locator(".np-commandbar button:has-text('Import')").click();
      } else if (dialog === "calculation_setup") {
        await page.locator(".np-menu button:has-text('Calculate')").click();
        await page.locator("button:has-text('Setup Calculation')").first().click();
      } else if (dialog === "method_scope") {
        await page.locator(".np-menu button:has-text('Tools')").click();
        await page.locator("button:has-text('Method Scope')").first().click();
      } else if (dialog === "export_options") {
        await page.locator(".np-menu button:has-text('Report')").click();
        await page.locator("button:has-text('Export Options')").first().click();
      } else if (dialog === "help_shortcuts") {
        await page.locator(".np-menu button:has-text('Help')").click();
        await page.locator("button:has-text('Shortcuts')").first().click();
      } else {
        const opener = {
          new_project: "New Project",
          sample_gallery: "Sample Project",
        }[dialog];
        await page.locator(`button:has-text("${opener}")`).first().click();
      }
      await page.waitForSelector(`[data-v237-dialog='${dialog}']`, { timeout: 10_000 });
      await page.screenshot({ path: path.join(SCREEN_DIR, `${viewport.name}-dialog-${dialog}.png`), fullPage: false });
      dialogSnapshots[dialog] = await page.evaluate((id) => Boolean(document.querySelector(`[data-v237-dialog='${id}']`)), dialog);
      await page.locator("[aria-label='Close dialog']").click();
    }

    const shell = await page.evaluate(() => {
      const text = document.body.innerText;
      return {
        hasShell: Boolean(document.querySelector("[data-v237-native-prototype='true']")),
        menus: [...document.querySelectorAll(".np-menu-slot > button")].map((node) => node.textContent?.trim()),
        rail: [...document.querySelectorAll(".np-rail button")].map((node) => node.textContent?.replace(/\s+/g, " ").trim()),
        hasStatusBar: Boolean(document.querySelector(".np-statusbar")),
        noMojibake: !text.includes("RÃ") && !text.includes("Ãƒ") && !text.includes("ï¿½"),
        noSmartPlsEquivalence: !/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(text),
      };
    });
    return { viewport: viewport.name, snapshots, dialogSnapshots, shell, consoleErrors: errors };
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

await ensureDir(SCREEN_DIR);
const runs = [];
for (let index = 0; index < viewports.length; index += 1) {
  runs.push(await runViewport(viewports[index], index));
}

const checks = {
  all_views_render: runs.every((run) => views.every((view) => run.snapshots[view]?.present)),
  all_dialogs_render: runs.every((run) => dialogs.every((dialog) => run.dialogSnapshots[dialog])),
  native_shell_present: runs.every((run) => run.shell.hasShell && run.shell.hasStatusBar),
  menu_contract_present: runs.every((run) => ["File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Tools", "Window", "Help"].every((menu) => run.shell.menus.includes(menu))),
  rail_contract_present: runs.every((run) => ["Home", "Data", "Model", "Setup", "Run", "Results", "Report", "Trust Center", "Settings"].every((item) => run.shell.rail.some((label) => label.includes(item)))),
  no_console_errors: runs.every((run) => run.consoleErrors.length === 0),
  no_horizontal_overflow: runs.every((run) => Object.values(run.snapshots).every((snapshot) => snapshot.noHorizontalOverflow)),
  no_mojibake: runs.every((run) => run.shell.noMojibake),
  no_smartpls_equivalence_claim: runs.every((run) => run.shell.noSmartPlsEquivalence),
};

const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));

const payload = {
  passed: issues.length === 0,
  milestone: "v2_37_0_native_frontend_prototype_shell",
  generatedAt: new Date().toISOString(),
  checks,
  issues,
  runs,
};

await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.37 native frontend prototype shell screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
