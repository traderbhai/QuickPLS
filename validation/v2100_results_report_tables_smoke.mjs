import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const TARGET = "v2_10_0_results_report_research_table_pass";
const ARTIFACTS = path.join(RESULTS, "screens", "v2100", "results-report-tables");
const OUTPUT = path.join(RESULTS, "v2100_results_report_tables_smoke.json");

await ensureDir(ARTIFACTS);

async function prepareCompletedRun(page) {
  await page.evaluate(() => {
    window.__QUICKPLS_SMOKE__?.addCompletedRun?.();
    window.__QUICKPLS_SMOKE__?.addComparisonRun?.();
  });
  await page.waitForTimeout(250);
}

async function collectResultsState(page, dir) {
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView?.("runs"));
  await page.waitForTimeout(250);
  const screenshot = path.join(dir, "results-overview.png");
  await page.screenshot({ path: screenshot, fullPage: true });
  const shell = await collectV2ShellSnapshot(page);
  const state = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    const researchTables = [...document.querySelectorAll("[data-results-research-table-pass='v2.10']")];
    const captions = [...document.querySelectorAll(".v2100-research-table caption")].map((node) => node.textContent?.replace(/\s+/g, " ").trim() ?? "");
    const affordances = [...document.querySelectorAll(".v2100-table-affordance")].map((node) => node.textContent?.replace(/\s+/g, " ").trim() ?? "");
    const exportButtons = [...document.querySelectorAll("button")].filter((button) => button.textContent?.includes("Export table"));
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      heading: document.querySelector(".workspace-page h1")?.textContent?.trim() ?? "",
      researchTableCount: researchTables.length,
      tableTitles: researchTables.map((node) => node.getAttribute("data-result-table-title") ?? ""),
      captions,
      affordances,
      exportButtonCount: exportButtons.length,
      hasRunSpecificFindings: document.body.innerText.includes("Run-specific findings"),
      hasSelectedRun: document.body.innerText.includes("PLS path modeling core run"),
      documentOverflowX: root.scrollWidth - root.clientWidth,
    };
  });
  return { screenshot, state: { ...shell, ...state } };
}

async function collectReportState(page, dir) {
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView?.("reports"));
  await page.waitForTimeout(250);
  const screenshot = path.join(dir, "report-preview.png");
  await page.screenshot({ path: screenshot, fullPage: true });
  const shell = await collectV2ShellSnapshot(page);
  const state = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    const workspace = document.querySelector("[data-report-export-flow='v2.10']");
    const previews = [...document.querySelectorAll("[data-report-table-preview='v2.10']")];
    const exportButtons = [...document.querySelectorAll(".v2100-report-table-preview button")].map((node) => node.textContent?.replace(/\s+/g, " ").trim() ?? "");
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      heading: document.querySelector(".workspace-page h1")?.textContent?.trim() ?? "",
      hasReportFlowMarker: Boolean(workspace),
      reportPreviewCount: previews.length,
      previewTitles: previews.map((node) => node.querySelector("h3")?.textContent?.trim() ?? ""),
      previewExportButtons: exportButtons.filter((text) => text.includes("Export table")).length,
      hasExportStatusFeedback: document.body.innerText.includes("No export has been created yet") || document.body.innerText.includes("export"),
      documentOverflowX: root.scrollWidth - root.clientWidth,
    };
  });
  return { screenshot, state: { ...shell, ...state } };
}

async function captureViewport({ viewport, port, label }) {
  const dir = path.join(ARTIFACTS, label);
  await ensureDir(dir);
  return withPreviewPage({
    port,
    viewport,
    run: async ({ page, errors }) => {
      await prepareCompletedRun(page);
      const results = await collectResultsState(page, dir);
      const report = await collectReportState(page, dir);
      const checklist = {
        completed_run_loaded: results.state.hasSelectedRun,
        results_has_research_tables: results.state.researchTableCount >= 3,
        results_tables_have_captions: results.state.captions.length >= 3 && results.state.captions.every(Boolean),
        results_tables_have_scan_affordance: results.state.affordances.length >= 3,
        results_tables_have_export_buttons: results.state.exportButtonCount >= 3,
        report_has_v2100_flow_marker: report.state.hasReportFlowMarker,
        report_has_table_previews: report.state.reportPreviewCount >= 1,
        report_previews_have_export_buttons: report.state.previewExportButtons >= 1,
        no_horizontal_document_overflow: Math.max(results.state.documentOverflowX, report.state.documentOverflowX) <= 2,
        no_rendered_mojibake: !results.state.hasR2Mojibake && !report.state.hasR2Mojibake,
        no_smartpls_equivalence_claim: !results.state.hasSmartPlsEquivalence && !report.state.hasSmartPlsEquivalence,
        no_console_errors: errors.length === 0,
      };
      return {
        viewport,
        screenshot_dir: dir,
        results,
        report,
        checklist,
        issues: issuesFromChecklist(checklist),
        errors,
      };
    },
  });
}

const runs = [];
for (const config of [
  { label: "1440x900", viewport: { width: 1440, height: 900 }, port: 53310 },
  { label: "1280x800", viewport: { width: 1280, height: 800 }, port: 53311 },
]) {
  runs.push(await captureViewport(config));
}

const issues = runs.flatMap((run) => [
  ...run.issues.map((issue) => ({ ...issue, viewport: `${run.viewport.width}x${run.viewport.height}` })),
  ...run.errors.map((error) => ({
    id: "console_error",
    severity: "high",
    viewport: `${run.viewport.width}x${run.viewport.height}`,
    detail: error,
  })),
]);

const result = {
  schema_version: 1,
  target: TARGET,
  passed: issues.length === 0,
  generated_at: new Date().toISOString(),
  screenshots_dir: ARTIFACTS,
  runs,
  issues,
};

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
