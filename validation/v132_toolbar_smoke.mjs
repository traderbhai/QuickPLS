import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v132", "toolbar");
const OUTPUT = path.join(RESULTS, "v132_toolbar_smoke.json");
const PORT = 53132;
const URL = `http://127.0.0.1:${PORT}/`;

await mkdir(ARTIFACTS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
let logs = "";
server.stdout.on("data", (data) => { logs += data.toString(); });
server.stderr.on("data", (data) => { logs += data.toString(); });

async function waitForUrl() {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(URL, { signal: AbortSignal.timeout(1500) });
      if (response.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Timed out waiting for Vite preview. ${logs.slice(-1200)}`);
}

async function toolbarMetrics(page) {
  return page.evaluate(() => {
    const toolbar = document.querySelector(".canvas-toolbar");
    const primary = document.querySelector(".canvas-toolbar-primary");
    const canvas = document.querySelector(".model-canvas");
    const text = toolbar?.textContent ?? "";
    return {
      toolbar_width: toolbar?.clientWidth ?? 0,
      toolbar_scroll_width: toolbar?.scrollWidth ?? 0,
      primary_width: primary?.clientWidth ?? 0,
      primary_scroll_width: primary?.scrollWidth ?? 0,
      has_toolbar_scroll: Boolean(toolbar && toolbar.scrollWidth > toolbar.clientWidth + 2),
      has_primary_scroll: Boolean(primary && primary.scrollWidth > primary.clientWidth + 2),
      text,
      main_has_residual: text.includes("Residual"),
      main_has_caption: text.includes("Caption"),
      main_has_observed_indicator: text.includes("Observed indicator"),
      main_has_latent_tool: text.includes("Latent construct tool"),
      dropdown_count: document.querySelectorAll(".canvas-dropdown-menu").length,
      context_text: document.querySelector(".canvas-context-toolbar")?.textContent ?? "",
      canvas_class: canvas?.className ?? "",
      grid_visible: Boolean(document.querySelector(".react-flow__background")),
      minimap_visible: Boolean(document.querySelector(".react-flow__minimap")),
      disabled_buttons: [...document.querySelectorAll(".canvas-toolbar button:disabled")].map((button) => button.textContent?.trim() || button.getAttribute("aria-label") || ""),
    };
  });
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  const toolbar = page.locator(".canvas-toolbar");

  const shots = {};
  shots.default = path.join(ARTIFACTS, "01_default_toolbar_1440.png");
  await page.screenshot({ path: shots.default, fullPage: true });
  const defaultMetrics = await toolbarMetrics(page);

  await toolbar.getByRole("button", { name: /^Arrange$/i }).click();
  const arrangeText = await page.locator(".canvas-dropdown-menu").innerText();
  shots.arrange = path.join(ARTIFACTS, "02_arrange_menu.png");
  await page.screenshot({ path: shots.arrange, fullPage: true });
  await page.keyboard.press("Escape");

  await toolbar.getByRole("button", { name: /^View$/i }).click();
  const viewText = await page.locator(".canvas-dropdown-menu").innerText();
  shots.view = path.join(ARTIFACTS, "03_view_menu.png");
  await page.screenshot({ path: shots.view, fullPage: true });
  await page.getByRole("menuitem", { name: /Academic grayscale/i }).click();
  const themeMetrics = await toolbarMetrics(page);
  await page.getByRole("menuitem", { name: /Hide grid/i }).click();
  const gridHiddenMetrics = await toolbarMetrics(page);
  await page.getByRole("menuitem", { name: /^Lock layout$/i }).click();
  const lockedMetrics = await toolbarMetrics(page);
  await page.getByRole("menuitem", { name: /^Unlock layout$/i }).click();
  await page.keyboard.press("Escape");

  await toolbar.getByRole("button", { name: /^Results$/i }).click();
  const resultsText = await page.locator(".canvas-dropdown-menu").innerText();
  shots.results = path.join(ARTIFACTS, "04_results_menu.png");
  await page.screenshot({ path: shots.results, fullPage: true });
  await page.keyboard.press("Escape");

  await page.locator(".smartpls-latent-node").first().click();
  const constructMetrics = await toolbarMetrics(page);
  shots.construct = path.join(ARTIFACTS, "05_construct_context.png");
  await page.screenshot({ path: shots.construct, fullPage: true });

  await page.locator(".smartpls-indicator-node").first().click();
  const indicatorMetrics = await toolbarMetrics(page);
  shots.indicator = path.join(ARTIFACTS, "06_indicator_context.png");
  await page.screenshot({ path: shots.indicator, fullPage: true });

  await page.locator(".sem-edge-label").first().click();
  const pathMetrics = await toolbarMetrics(page);
  shots.path = path.join(ARTIFACTS, "07_path_context.png");
  await page.screenshot({ path: shots.path, fullPage: true });

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.selectConstructs(["competence", "likeability", "satisfaction"]));
  await page.waitForTimeout(100);
  const multiMetrics = await toolbarMetrics(page);
  shots.multi = path.join(ARTIFACTS, "08_multi_selection_context.png");
  await page.screenshot({ path: shots.multi, fullPage: true });

  await page.setViewportSize({ width: 1280, height: 800 });
  await page.waitForTimeout(250);
  const narrowMetrics = await toolbarMetrics(page);
  shots.narrow = path.join(ARTIFACTS, "09_toolbar_1280.png");
  await page.screenshot({ path: shots.narrow, fullPage: true });

  const checklist = {
    toolbar_1440_has_no_horizontal_scroll: !defaultMetrics.has_toolbar_scroll,
    toolbar_1280_has_no_horizontal_scroll: !narrowMetrics.has_toolbar_scroll,
    main_toolbar_keeps_core_actions: ["Select", "Pan", "Construct", "Path", "Cov", "Arrange", "Fit", "Validate", "View", "Results"].every((text) => defaultMetrics.text.includes(text)),
    low_value_tools_hidden_from_main_toolbar: !defaultMetrics.main_has_residual && !defaultMetrics.main_has_caption && !defaultMetrics.main_has_observed_indicator && !defaultMetrics.main_has_latent_tool,
    arrange_menu_contains_required_options: ["Arrange like SmartPLS", "Left to right", "Top to bottom", "CFA measurement", "Mediation", "Large model"].every((text) => arrangeText.includes(text)),
    view_menu_contains_modes_and_preferences: ["Edit model", "Result diagram", "Compact", "Publication preview", "SmartPLS-like theme", "Academic grayscale", "QuickPLS color", "Journal mono", "High contrast", "Lock layout", "Hide grid"].every((text) => viewText.includes(text)),
    view_theme_button_changes_canvas_class: themeMetrics.canvas_class.includes("theme-academic_grayscale"),
    view_grid_button_hides_grid_and_minimap: !gridHiddenMetrics.grid_visible && !gridHiddenMetrics.minimap_visible,
    view_lock_button_disables_layout_actions: lockedMetrics.canvas_class.includes("layout-locked-canvas") && ["Construct", "Path", "Cov", "Auto indicators", "Reset indicator layout"].every((text) => lockedMetrics.disabled_buttons.includes(text)),
    results_menu_contains_overlay_controls: ["Run", "Overlay", "Precision", "Loadings", "Path coefficients", "R²", "Significance"].every((text) => resultsText.includes(text)),
    construct_context_toolbar_visible: ["Construct:", "Rename", "Duplicate", "Auto indicators", "Delete"].every((text) => constructMetrics.context_text.includes(text)),
    indicator_context_toolbar_visible: ["Indicator:", "Rename", "Reassign", "Left", "Right", "Reset position", "Unassign"].every((text) => indicatorMetrics.context_text.includes(text)),
    path_context_toolbar_visible: ["Path:", "Reverse", "Straight", "Curved", "Orthogonal", "Reset label", "Delete"].every((text) => pathMetrics.context_text.includes(text)),
    multi_selection_toolbar_visible: ["3 constructs selected", "Left", "Center X", "Top", "Center Y", "Distribute H", "Distribute V", "Tidy selection"].every((text) => multiMetrics.context_text.includes(text)),
    screenshots_written: Object.values(shots).every(Boolean),
  };

  const report = {
    schema_version: 1,
    target: "QuickPLS v1.3.2 SEM canvas toolbar redesign",
    passed: errors.length === 0 && Object.values(checklist).every(Boolean),
    checklist,
    errors,
    evidence: { defaultMetrics, narrowMetrics, arrangeText, viewText, resultsText, themeMetrics, gridHiddenMetrics, lockedMetrics, constructContext: constructMetrics.context_text, indicatorContext: indicatorMetrics.context_text, pathContext: pathMetrics.context_text, multiContext: multiMetrics.context_text, screenshots: shots },
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
