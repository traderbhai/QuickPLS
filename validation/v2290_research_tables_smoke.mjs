import fs from "node:fs/promises";
import path from "node:path";
import { ensureDir, RESULTS, withPreviewPage, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2290", "research-tables");
const OUTPUT = path.join(RESULTS, "v2290_research_tables_smoke.json");
const viewports = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "desktop-1280", width: 1280, height: 800 },
];

async function tableSnapshot(page) {
  return page.evaluate(() => {
    const tables = [...document.querySelectorAll("[data-v229-research-table='true']")];
    const firstTable = tables[0];
    const firstHeader = firstTable?.querySelector(".bootstrap-table-scroll th:first-child");
    const secondHeader = firstTable?.querySelector(".bootstrap-table-scroll th:nth-child(2)");
    const body = document.body.innerText;
    const rect = document.documentElement.getBoundingClientRect();
    return {
      tableCount: tables.length,
      titles: tables.map((table) => table.getAttribute("data-result-table-title") ?? ""),
      hasToolbar: Boolean(firstTable?.querySelector(".research-table-toolbar")),
      hasLocalSearch: Boolean(firstTable?.querySelector(".research-table-toolbar input[aria-label^='Search ']")),
      hasPrecision: Boolean(firstTable?.querySelector(".research-table-toolbar select[aria-label^='Precision ']")),
      hasCopySelected: body.includes("Copy selected"),
      hasCopyTable: body.includes("Copy table"),
      hasExportTable: body.includes("Export table"),
      hasSelectAll: Boolean(firstTable?.querySelector(".research-table-select-cell input[aria-label^='Select all visible ']")),
      hasRowCheckbox: Boolean(firstTable?.querySelector("tbody .research-table-select-cell input")),
      firstColumnSticky: firstHeader ? getComputedStyle(firstHeader).position === "sticky" : false,
      firstDataColumnSticky: secondHeader ? getComputedStyle(secondHeader).position === "sticky" : false,
      rowDetailVisible: /Selected row|Row detail/i.test(body),
      sortedByVisible: body.includes("Sorted by"),
      selectedCountVisible: /selected row/i.test(body),
      noR2Mojibake: !body.includes("RÃ‚Â²") && !body.includes("RÃƒ") && !body.includes("Ãƒâ€š"),
      horizontalOverflow: document.documentElement.scrollWidth > Math.ceil(rect.width) + 4,
    };
  });
}

async function clickResultTab(page, label) {
  await page.evaluate((target) => {
    const button = [...document.querySelectorAll(".results-section-nav button")]
      .find((node) => node.textContent?.toLowerCase().includes(String(target).toLowerCase()));
    if (button instanceof HTMLButtonElement) button.click();
  }, label);
}

async function runViewport(viewport, index) {
  const port = 57290 + index;
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
      await page.waitForSelector("[data-result-table-title='Path coefficients']", { timeout: 10_000 });
      const overviewBefore = await tableSnapshot(page);
      const overviewShot = path.join(SCREEN_DIR, `${viewport.name}-overview-tables.png`);
      await page.screenshot({ path: overviewShot, fullPage: false });

      const interactionTable = page.locator("[data-result-table-title='Total effects']").first();
      await interactionTable.locator("tbody tr").first().click();
      await page.evaluate(() => {
        const checkbox = document.querySelector("[data-result-table-title='Total effects'] tbody .research-table-select-cell input");
        if (checkbox instanceof HTMLInputElement) checkbox.click();
      });
      await page.evaluate(() => {
        const input = document.querySelector("[data-result-table-title='Total effects'] .research-table-toolbar input");
        if (input instanceof HTMLInputElement) {
          input.value = "loyalty";
          input.dispatchEvent(new Event("input", { bubbles: true }));
          input.dispatchEvent(new Event("change", { bubbles: true }));
        }
        const sortButton = document.querySelectorAll("[data-result-table-title='Total effects'] .table-sort-button")[1];
        if (sortButton instanceof HTMLButtonElement) sortButton.click();
      });
      const overviewAfter = await tableSnapshot(page);

      await clickResultTab(page, "Validity");
      await page.waitForSelector("[data-result-table-title='HTMT+ construct pairs']", { state: "attached", timeout: 10_000 });
      const validity = await tableSnapshot(page);
      const validityShot = path.join(SCREEN_DIR, `${viewport.name}-validity-htmt.png`);
      await page.screenshot({ path: validityShot, fullPage: false });

      await clickResultTab(page, "Inference");
      await page.waitForSelector("[data-result-table-title='Bootstrap estimates']", { state: "attached", timeout: 10_000 });
      const inference = await tableSnapshot(page);
      const inferenceShot = path.join(SCREEN_DIR, `${viewport.name}-inference-bootstrap.png`);
      await page.screenshot({ path: inferenceShot, fullPage: false });

      await clickResultTab(page, "Structural");
      await page.waitForSelector("[data-result-table-title='Cohen f² effect sizes']", { state: "attached", timeout: 10_000 });
      const structural = await tableSnapshot(page);

      return {
        viewport: viewport.name,
        overviewBefore,
        overviewAfter,
        validity,
        inference,
        structural,
        screenshots: [overviewShot, validityShot, inferenceShot],
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
  research_table_marker_present: runs.every((run) => run.overviewBefore.tableCount >= 2),
  toolbar_controls_present: runs.every((run) => run.overviewBefore.hasToolbar && run.overviewBefore.hasLocalSearch && run.overviewBefore.hasPrecision),
  table_actions_present: runs.every((run) => run.overviewBefore.hasCopySelected && run.overviewBefore.hasCopyTable && run.overviewBefore.hasExportTable),
  row_selection_present: runs.every((run) => run.overviewBefore.hasSelectAll && run.overviewBefore.hasRowCheckbox),
  sticky_columns_present: runs.every((run) => run.overviewBefore.firstColumnSticky && run.overviewBefore.firstDataColumnSticky),
  row_detail_and_selection_work: runs.every((run) => run.overviewAfter.rowDetailVisible && run.overviewAfter.selectedCountVisible),
  sorting_feedback_visible: runs.every((run) => run.overviewAfter.sortedByVisible),
  validity_pair_table_present: runs.every((run) => run.validity.titles.includes("HTMT+ construct pairs")),
  bootstrap_tables_present: runs.every((run) => run.inference.titles.includes("Bootstrap estimates") && run.inference.titles.includes("Percentile confidence intervals")),
  structural_tables_present: runs.every((run) => run.structural.titles.includes("Cohen f² effect sizes")),
  no_console_errors: runs.every((run) => run.consoleErrors.length === 0),
  no_r2_mojibake: runs.every((run) => run.overviewBefore.noR2Mojibake && run.validity.noR2Mojibake && run.inference.noR2Mojibake),
  no_document_horizontal_overflow: runs.every((run) => !run.overviewBefore.horizontalOverflow && !run.validity.horizontalOverflow && !run.inference.horizontalOverflow),
};

const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));

const payload = {
  passed: issues.length === 0,
  milestone: "v2_29_0_research_table_system",
  generatedAt: new Date().toISOString(),
  browserPlugin: "absent; Playwright smoke harness used",
  checks,
  issues,
  runs,
};

await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.29 research table system smoke screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
