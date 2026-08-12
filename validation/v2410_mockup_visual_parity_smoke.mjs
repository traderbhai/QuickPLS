import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { ensureDir, RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const MANIFEST_PATH = path.join("validation", "mockups", "v2410_mockup_manifest.json");
const SCREEN_DIR = path.join(RESULTS, "screens", "v2410", "mockup-parity");
const OUTPUT = path.join(RESULTS, "v2410_mockup_visual_parity_smoke.json");

const railLabels = {
  home: "Home",
  data: "Data",
  model: "Model",
  setup: "Setup",
  run: "Run",
  results: "Results",
  report: "Report",
  trust: "Trust Center",
  settings: "Settings",
};

const dialogOpeners = {
  new_project: ["File", "New Project"],
  import_data: ["Data", "Import Data"],
  calculation_setup: ["Calculate", "Setup Calculation"],
  method_scope: ["Tools", "Method Scope"],
  export_options: ["Report", "Export Options"],
  help_shortcuts: ["Help", "Shortcuts"],
  settings: ["Tools", "Preferences"],
};

const slug = (value) => value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

async function clickMenuItem(page, menu, item) {
  await page.locator(`.np-menu > .np-menu-slot > button:has-text("${menu}")`).click();
  await page.locator(`.np-menu-popover button:has-text("${item}")`).click();
}

async function openView(page, view) {
  const label = railLabels[view];
  if (!label) throw new Error(`Unknown view in manifest: ${view}`);
  await page.locator(`.np-rail button:has-text("${label}")`).click();
  await page.waitForSelector(`[data-v237-screen='${view}']`, { timeout: 10_000 });
  await page.waitForTimeout(120);
}

async function run() {
  const manifest = JSON.parse(await fs.readFile(MANIFEST_PATH, "utf-8"));
  const port = 57641;
  const url = `http://127.0.0.1:${port}/`;
  const { server, logs } = startPreview(port);
  let browser;
  try {
    await waitForPreview(url, logs);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: manifest.viewport, deviceScaleFactor: 1 });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${url}?native_shell=1&quickpls_smoke=1&mockup_parity=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForSelector("[data-v237-native-prototype='true'][data-v241-mockup-parity='true']", { timeout: 10_000 });

    const states = [];
    for (const state of manifest.states) {
      await openView(page, state.view);
      if (state.kind === "dialog") {
        const opener = dialogOpeners[state.dialog];
        if (!opener) throw new Error(`No opener for dialog ${state.dialog}`);
        await clickMenuItem(page, opener[0], opener[1]);
        await page.waitForSelector(`[data-v237-dialog='${state.dialog}']`, { timeout: 10_000 });
      }
      const screenshot = path.join(SCREEN_DIR, `${slug(state.id)}.png`);
      await page.screenshot({ path: screenshot, fullPage: false });
      const text = await page.evaluate(() => {
        const visibleText = document.body.innerText;
        const fieldValues = Array.from(document.querySelectorAll("input, textarea, select"))
          .map((field) => field.value ?? "")
          .join(" ");
        return `${visibleText} ${fieldValues}`.replace(/\s+/g, " ").trim();
      });
      const metrics = await page.evaluate(() => ({
        root: Boolean(document.querySelector("[data-v241-mockup-parity='true']")),
        ribbonButtons: document.querySelectorAll(".np-ribbon-group button").length,
        measurementEdges: document.querySelectorAll(".np-measurement-line").length,
        structuralEdges: document.querySelectorAll(".np-structural-line").length,
        covarianceEdges: document.querySelectorAll(".np-covariance-line").length,
        panels: document.querySelectorAll(".np-panel, .np-main-pane, .np-inspector, .np-tree-pane, .np-canvas-pane").length,
        noHorizontalOverflow: document.documentElement.scrollWidth <= window.innerWidth + 4,
        noMojibake: !/(RÃ|Ãƒ|Ã‚|ï¿½|�)/.test(document.body.innerText),
      }));
      const missingRequired = (state.required ?? []).filter((needle) => !text.includes(needle));
      states.push({
        id: state.id,
        kind: state.kind,
        view: state.view,
        dialog: state.dialog ?? null,
        mockup: path.join(manifest.mockup_dir, state.file),
        screenshot,
        missingRequired,
        metrics,
      });
      if (state.kind === "dialog") {
        await page.keyboard.press("Escape");
        await page.locator(`[data-v237-dialog='${state.dialog}'] button[aria-label='Close dialog']`).click().catch(async () => {
          await page.locator(`[data-v237-dialog='${state.dialog}'] button:has-text("Cancel")`).click();
        });
        await page.waitForTimeout(80);
      }
    }

    const shell = await page.evaluate(() => {
      const text = document.body.innerText;
      return {
        title: document.querySelector(".np-titlebar strong")?.textContent ?? "",
        hasParityMarker: Boolean(document.querySelector("[data-v241-mockup-parity='true']")),
        hasDesktopMenu: document.querySelectorAll(".np-menu-slot").length >= 10,
        hasRibbon: Boolean(document.querySelector(".np-ribbon")),
        hasRailSupport: text.includes("Trust Center") && text.includes("Settings"),
        noSmartPlsEquivalence: !/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(text),
      };
    });

    const checks = {
      parity_marker_present: shell.hasParityMarker,
      desktop_menu_present: shell.hasDesktopMenu,
      ribbon_present: shell.hasRibbon,
      support_rail_present: shell.hasRailSupport,
      all_states_captured: states.length === manifest.states.length,
      all_required_text_present: states.every((state) => state.missingRequired.length === 0),
      all_states_no_horizontal_overflow: states.every((state) => state.metrics.noHorizontalOverflow),
      no_state_mojibake: states.every((state) => state.metrics.noMojibake),
      model_measurement_edges_present: states.find((state) => state.id === "model")?.metrics.measurementEdges >= 15,
      model_structural_edges_present: states.find((state) => state.id === "model")?.metrics.structuralEdges >= 6,
      model_covariance_edge_present: states.find((state) => state.id === "model")?.metrics.covarianceEdges >= 1,
      no_console_errors: errors.length === 0,
      no_smartpls_equivalence_claim: shell.noSmartPlsEquivalence,
    };
    const issues = Object.entries(checks)
      .filter(([, passed]) => !passed)
      .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));
    return { manifest, shell, states, consoleErrors: errors, checks, issues, passed: issues.length === 0 };
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

await ensureDir(SCREEN_DIR);
const result = await run();
const payload = {
  passed: result.passed,
  milestone: "v2_41_0_full_mockup_screen_parity_pass",
  generatedAt: new Date().toISOString(),
  checks: result.checks,
  issues: result.issues,
  shell: result.shell,
  states: result.states,
  consoleErrors: result.consoleErrors,
};
await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.41 mockup parity screenshot captures. Compare each PNG against its manifest mockup path.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
