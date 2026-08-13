import fs from "node:fs/promises";
import path from "node:path";
import { ensureDir, RESULTS, withPreviewPage, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2310", "report-export-wizard");
const OUTPUT = path.join(RESULTS, "v2310_report_export_wizard_smoke.json");
const viewports = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "desktop-1280", width: 1280, height: 800 },
];

async function clickWizardStep(page, step) {
  await page.locator(`[data-v2310-step-button='${step}']`).click();
  await page.waitForSelector(`[data-v2310-wizard-step='${step}']`, { timeout: 10_000 });
}

async function wizardSnapshot(page) {
  return page.evaluate(() => {
    const text = document.body.innerText;
    const root = document.querySelector("[data-v2310-report-wizard='true']");
    const activePane = document.querySelector("[data-v2310-wizard-step]");
    const buttons = [...document.querySelectorAll("[data-v2310-step-button]")];
    const rect = document.documentElement.getBoundingClientRect();
    return {
      hasWizard: Boolean(root),
      activeStep: activePane?.getAttribute("data-v2310-wizard-step") ?? "",
      stepButtons: buttons.map((button) => button.getAttribute("data-v2310-step-button") ?? ""),
      hasPresetCards: text.includes("Reviewer pack") && text.includes("Journal figure"),
      hasDiagramPreview: Boolean(document.querySelector(".publication-preview-frame")),
      hasTablePreview: Boolean(document.querySelector("[data-report-table-preview='v2.10']")),
      hasSettingsGroups: text.includes("Figure settings") && text.includes("Table settings") && text.includes("Notes and interpretation") && text.includes("Provenance and reviewer pack"),
      hasExportActions: ["CSV tables", "HTML report", "XLSX workbook", "Print / PDF", "Model diagram SVG"].every((label) => text.includes(label)),
      hasDisabledReasons: text.includes("Desktop runtime required") || text.includes("Run a method before"),
      comparisonMovedToResults: text.includes("Open Results Comparison"),
      hasStatusFeedbackSurface: Boolean(document.querySelector(".export-status-feedback")) || text.includes("Each action shows its enabled state"),
      noR2Mojibake: !text.includes("RÃ") && !text.includes("RÂ"),
      noHorizontalOverflow: document.documentElement.scrollWidth <= Math.ceil(rect.width) + 4,
    };
  });
}

async function runViewport(viewport, index) {
  const port = 57400 + index;
  return withPreviewPage({
    port,
    viewport: { width: viewport.width, height: viewport.height },
    run: async ({ page, errors }) => {
      await page.evaluate(() => {
        window.__QUICKPLS_SMOKE__?.loadDiagramFixture?.("mediation");
        window.__QUICKPLS_SMOKE__?.addCompletedRun?.();
        window.__QUICKPLS_SMOKE__?.setView?.("reports");
      });
      await page.waitForSelector("[data-v2310-report-wizard='true']", { timeout: 10_000 });

      const states = {};
      for (const step of ["content", "preview", "settings", "export"]) {
        await clickWizardStep(page, step);
        states[step] = await wizardSnapshot(page);
        await page.screenshot({ path: path.join(SCREEN_DIR, `${viewport.name}-${step}.png`), fullPage: false });
      }

      return { viewport: viewport.name, states, consoleErrors: errors };
    },
  });
}

await ensureDir(SCREEN_DIR);
const runs = [];
for (let index = 0; index < viewports.length; index += 1) {
  runs.push(await runViewport(viewports[index], index));
}

const checks = {
  wizard_present: runs.every((run) => Object.values(run.states).every((state) => state.hasWizard)),
  four_steps_present: runs.every((run) => Object.values(run.states).every((state) => ["content", "preview", "settings", "export"].every((step) => state.stepButtons.includes(step)))),
  content_step_has_presets: runs.every((run) => run.states.content.hasPresetCards),
  preview_step_has_figure_and_tables: runs.every((run) => run.states.preview.hasDiagramPreview && run.states.preview.hasTablePreview),
  settings_step_has_grouped_controls: runs.every((run) => run.states.settings.hasSettingsGroups),
  export_step_has_explicit_outputs: runs.every((run) => run.states.export.hasExportActions && run.states.export.hasDisabledReasons),
  comparison_lives_in_results: runs.every((run) => run.states.export.comparisonMovedToResults),
  status_feedback_surface_present: runs.every((run) => run.states.export.hasStatusFeedbackSurface),
  no_console_errors: runs.every((run) => run.consoleErrors.length === 0),
  no_r2_mojibake: runs.every((run) => Object.values(run.states).every((state) => state.noR2Mojibake)),
  no_document_horizontal_overflow: runs.every((run) => Object.values(run.states).every((state) => state.noHorizontalOverflow)),
};

const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));

const payload = {
  passed: issues.length === 0,
  milestone: "v2_31_0_report_export_wizard",
  generatedAt: new Date().toISOString(),
  browserPlugin: "absent; Playwright smoke harness used",
  checks,
  issues,
  runs,
};

await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.31 report export wizard smoke screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
