import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { ensureDir, RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2390", "screen-replacement");
const OUTPUT = path.join(RESULTS, "v2390_native_frontend_screen_replacement_smoke.json");

const viewPairs = [
  ["Home", "home", "welcome"],
  ["Data", "data", "data"],
  ["Model", "model", "models"],
  ["Setup", "setup", "analyses"],
  ["Run", "run", "run"],
  ["Results", "results", "runs"],
  ["Report", "report", "reports"],
  ["Trust Center", "trust", "trust"],
  ["Settings", "settings", "settings"],
];

async function run() {
  const port = 57639;
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
    await page.waitForSelector("[data-v237-native-prototype='true'][data-v239-shell-mode='production-candidate']", { timeout: 10_000 });

    const snapshots = {};
    for (const [label, nativeView, workspaceView] of viewPairs) {
      await page.locator(`.np-rail button:has-text("${label}")`).click();
      await page.waitForSelector(`[data-v237-screen='${nativeView}']`, { timeout: 10_000 });
      await page.waitForFunction((expected) => {
        const root = document.querySelector("[data-v237-native-prototype='true']");
        return root?.getAttribute("data-v239-workspace-view") === expected;
      }, workspaceView, { timeout: 10_000 });
      await page.screenshot({ path: path.join(SCREEN_DIR, `${nativeView}.png`), fullPage: false });
      snapshots[nativeView] = await page.evaluate((screen) => {
        const root = document.querySelector(`[data-v237-screen='${screen}']`);
        return {
          present: Boolean(root),
          text: root?.textContent?.replace(/\s+/g, " ").slice(0, 1200) ?? "",
          workspaceView: document.querySelector("[data-v237-native-prototype='true']")?.getAttribute("data-v239-workspace-view") ?? "",
          noHorizontalOverflow: document.documentElement.scrollWidth <= window.innerWidth + 4,
        };
      }, nativeView);
    }

    const shell = await page.evaluate(() => {
      const root = document.querySelector("[data-v237-native-prototype='true']");
      const body = document.body.innerText;
      return {
        mode: root?.getAttribute("data-v239-shell-mode"),
        adapter: root?.getAttribute("data-v238-adapter"),
        hasLegacyAppChrome: Boolean(document.querySelector(".app-shell, .top-bar, .workflow-strip")),
        hasNativeMenu: Boolean(document.querySelector(".np-menu")),
        hasNativeStatusbar: Boolean(document.querySelector(".np-statusbar")),
        noMojibake: !body.includes("RÃ") && !body.includes("Â²"),
      };
    });

    return { shell, snapshots, consoleErrors: errors };
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

await ensureDir(SCREEN_DIR);
const result = await run();
const checks = {
  candidate_shell_mode: result.shell.mode === "production-candidate",
  adapter_still_active: result.shell.adapter === "store",
  native_menu_present: result.shell.hasNativeMenu,
  native_statusbar_present: result.shell.hasNativeStatusbar,
  legacy_chrome_not_mounted: !result.shell.hasLegacyAppChrome,
  all_views_render: viewPairs.every(([, nativeView]) => result.snapshots[nativeView]?.present),
  workspace_routes_synced: viewPairs.every(([, nativeView, workspaceView]) => result.snapshots[nativeView]?.workspaceView === workspaceView),
  no_console_errors: result.consoleErrors.length === 0,
  no_horizontal_overflow: Object.values(result.snapshots).every((snapshot) => snapshot.noHorizontalOverflow),
  no_mojibake: result.shell.noMojibake,
};
const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));
const payload = {
  passed: issues.length === 0,
  milestone: "v2_39_0_native_frontend_screen_replacement_plan",
  generatedAt: new Date().toISOString(),
  checks,
  issues,
  result,
};
await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.39 native production-candidate shell route screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
