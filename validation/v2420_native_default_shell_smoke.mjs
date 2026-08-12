import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { ensureDir, RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const MILESTONE = "v2_42_0_make_native_mockup_shell_default";
const SCREEN_DIR = path.join(RESULTS, "screens", "v2420", "native-default");
const OUTPUT = path.join(RESULTS, "v2420_native_default_shell_smoke.json");

const requiredScreens = [
  ["Home", "home"],
  ["Data", "data"],
  ["Model", "model"],
  ["Setup", "setup"],
  ["Run", "run"],
  ["Results", "results"],
  ["Report", "report"],
  ["Trust Center", "trust"],
  ["Settings", "settings"],
];

async function captureDefaultShell(page) {
  await page.goto("http://127.0.0.1:57642/?quickpls_smoke=1", { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForSelector("[data-v237-native-prototype='true'][data-v239-shell-mode='production-candidate']", { timeout: 10_000 });
  const snapshots = {};
  for (const [label, screen] of requiredScreens) {
    await page.locator(`.np-rail button:has-text("${label}")`).click();
    await page.waitForSelector(`[data-v237-screen='${screen}']`, { timeout: 10_000 });
    await page.waitForTimeout(80);
    const screenshot = path.join(SCREEN_DIR, `${screen}.png`);
    await page.screenshot({ path: screenshot, fullPage: false });
    snapshots[screen] = await page.evaluate((screenName) => {
      const root = document.querySelector(`[data-v237-screen='${screenName}']`);
      return {
        present: Boolean(root),
        title: document.querySelector(".np-titlebar strong")?.textContent?.trim() ?? "",
        text: root?.textContent?.replace(/\s+/g, " ").slice(0, 1000) ?? "",
        noHorizontalOverflow: document.documentElement.scrollWidth <= window.innerWidth + 4,
      };
    }, screen);
  }
  const shell = await page.evaluate(() => {
    const root = document.querySelector("[data-v237-native-prototype='true']");
    const text = document.body.innerText;
    return {
      mode: root?.getAttribute("data-v239-shell-mode") ?? "",
      adapter: root?.getAttribute("data-v238-adapter") ?? "",
      hasNativeMenu: Boolean(document.querySelector(".np-menu")),
      hasNativeRibbon: Boolean(document.querySelector(".np-ribbon")),
      hasNativeRail: Boolean(document.querySelector(".np-rail")),
      hasNativeStatusbar: Boolean(document.querySelector(".np-statusbar")),
      hasLegacyChrome: Boolean(document.querySelector(".app-shell, .top-bar, .workflow-strip")),
      hasParityMarker: Boolean(document.querySelector("[data-v241-mockup-parity='true']")),
      noMojibake: !/(RÃƒ|ÃƒÆ’|Ãƒâ€š|Ã¯Â¿Â½|ï¿½|â€¢|Î”)/.test(text),
      noSmartPlsEquivalence: !/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(text),
    };
  });
  return { shell, snapshots };
}

async function captureLegacyFallback(page) {
  await page.goto("http://127.0.0.1:57642/?legacy_shell=1&quickpls_smoke=1", { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForSelector(".app-shell", { timeout: 10_000 });
  await page.screenshot({ path: path.join(SCREEN_DIR, "legacy-shell-fallback.png"), fullPage: false });
  return page.evaluate(() => ({
    hasLegacyShell: Boolean(document.querySelector(".app-shell")),
    hasNativeShell: Boolean(document.querySelector("[data-v237-native-prototype='true']")),
  }));
}

async function run() {
  const port = 57642;
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
    const defaultShell = await captureDefaultShell(page);
    const legacyFallback = await captureLegacyFallback(page);
    return { defaultShell, legacyFallback, consoleErrors: errors };
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

await ensureDir(SCREEN_DIR);
const result = await run();
const checks = {
  default_route_uses_native_shell: result.defaultShell.shell.mode === "production-candidate",
  default_route_uses_live_adapter: result.defaultShell.shell.adapter === "store",
  native_menu_present: result.defaultShell.shell.hasNativeMenu,
  native_ribbon_present: result.defaultShell.shell.hasNativeRibbon,
  native_rail_present: result.defaultShell.shell.hasNativeRail,
  native_statusbar_present: result.defaultShell.shell.hasNativeStatusbar,
  mockup_parity_marker_preserved: result.defaultShell.shell.hasParityMarker,
  legacy_chrome_not_mounted_by_default: !result.defaultShell.shell.hasLegacyChrome,
  all_default_screens_render: requiredScreens.every(([, screen]) => result.defaultShell.snapshots[screen]?.present),
  all_default_screens_no_horizontal_overflow: Object.values(result.defaultShell.snapshots).every((snapshot) => snapshot.noHorizontalOverflow),
  legacy_fallback_available: result.legacyFallback.hasLegacyShell && !result.legacyFallback.hasNativeShell,
  no_console_errors: result.consoleErrors.length === 0,
  no_mojibake: result.defaultShell.shell.noMojibake,
  no_smartpls_equivalence_claim: result.defaultShell.shell.noSmartPlsEquivalence,
};
const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));
const payload = {
  passed: issues.length === 0,
  milestone: MILESTONE,
  generatedAt: new Date().toISOString(),
  checks,
  issues,
  result,
};
await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.42 default native-shell screenshots. The legacy screenshot documents the explicit fallback route only.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
