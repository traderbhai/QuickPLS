import fs from "node:fs/promises";
import path from "node:path";
import { ensureDir, RESULTS, withPreviewPage, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2280", "results-workbook");
const OUTPUT = path.join(RESULTS, "v2280_results_workbook_smoke.json");
const viewports = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "desktop-1280", width: 1280, height: 800 },
];

async function snapshot(page) {
  return page.evaluate(() => {
    const root = document.querySelector('[data-v228-results-workbook="true"]');
    const body = document.body.innerText;
    const rect = document.documentElement.getBoundingClientRect();
    return {
      marker: Boolean(root),
      tabs: Array.from(document.querySelectorAll(".results-section-nav button")).map((node) => node.textContent?.replace(/\s+/g, " ").trim()).filter(Boolean),
      hasRunHeader: Boolean(document.querySelector(".results-v228-run-header")),
      hasWorkbookBody: Boolean(document.querySelector(".results-v228-workbook-body")),
      hasTableArea: Boolean(document.querySelector(".results-v228-table-area")),
      hasDetailPane: Boolean(document.querySelector(".results-v228-detail-pane")),
      hasConfidence: /method confidence/i.test(body),
      hasFindingsLanes: body.includes("Must address") && body.includes("Review") && body.includes("Info"),
      hasProvenanceFooter: Boolean(document.querySelector(".results-v228-provenance-footer")),
      hasComparisonTab: body.includes("Comparison"),
      hasR2Mojibake: body.includes("RÃ‚Â²") || body.includes("RÃƒ") || body.includes("Ãƒâ€š"),
      horizontalOverflow: document.documentElement.scrollWidth > Math.ceil(rect.width) + 4,
    };
  });
}

async function runViewport(viewport, index) {
  const port = 57280 + index;
  return withPreviewPage({
    port,
    viewport: { width: viewport.width, height: viewport.height },
    run: async ({ page, errors }) => {
      await page.evaluate(() => {
        window.__QUICKPLS_SMOKE__?.loadDiagramFixture?.("mediation");
        window.__QUICKPLS_SMOKE__?.addCompletedRun?.();
        window.__QUICKPLS_SMOKE__?.addComparisonRun?.();
        window.__QUICKPLS_SMOKE__?.setView?.("runs");
      });
      await page.waitForSelector('[data-v228-results-workbook="true"]', { timeout: 10_000 });
      await page.waitForSelector(".results-v228-detail-pane", { timeout: 10_000 });
      const overview = await snapshot(page);
      const overviewShot = path.join(SCREEN_DIR, `${viewport.name}-overview.png`);
      await page.screenshot({ path: overviewShot, fullPage: false });

      await page.getByRole("button", { name: /Comparison/i }).click();
      await page.waitForTimeout(150);
      const comparison = await snapshot(page);
      const comparisonShot = path.join(SCREEN_DIR, `${viewport.name}-comparison.png`);
      await page.screenshot({ path: comparisonShot, fullPage: false });

      return {
        viewport: viewport.name,
        overview,
        comparison,
        screenshots: [overviewShot, comparisonShot],
        consoleErrors: errors,
      };
    },
  });
}

await ensureDir(SCREEN_DIR);
const runs = [];
for (let index = 0; index < viewports.length; index += 1) {
  runs.push(await runViewport(viewports[index], index));
}

const checks = {
  workbook_marker_present: runs.every((run) => run.overview.marker && run.comparison.marker),
  run_header_present: runs.every((run) => run.overview.hasRunHeader),
  workbook_body_present: runs.every((run) => run.overview.hasWorkbookBody && run.overview.hasTableArea && run.overview.hasDetailPane),
  all_tabs_present: runs.every((run) => run.overview.tabs.length >= 10),
  method_confidence_present: runs.every((run) => run.overview.hasConfidence),
  findings_lanes_present: runs.every((run) => run.overview.hasFindingsLanes),
  provenance_footer_present: runs.every((run) => run.overview.hasProvenanceFooter),
  comparison_tab_rendered: runs.every((run) => run.comparison.hasComparisonTab),
  no_console_errors: runs.every((run) => run.consoleErrors.length === 0),
  no_r2_mojibake: runs.every((run) => !run.overview.hasR2Mojibake && !run.comparison.hasR2Mojibake),
  no_document_horizontal_overflow: runs.every((run) => !run.overview.horizontalOverflow && !run.comparison.horizontalOverflow),
};
const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));

const payload = {
  passed: issues.length === 0,
  milestone: "v2_28_0_results_workbook_redesign",
  generatedAt: new Date().toISOString(),
  browserPlugin: "absent; Playwright smoke harness used",
  checks,
  issues,
  runs,
};

await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.28 Results workbook smoke screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
