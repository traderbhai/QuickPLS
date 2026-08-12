import fs from "node:fs/promises";
import path from "node:path";
import { ensureDir, RESULTS, withPreviewPage, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const SCREEN_DIR = path.join(RESULTS, "screens", "v2300", "reportability-assistant");
const OUTPUT = path.join(RESULTS, "v2300_reportability_assistant_smoke.json");
const viewports = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "desktop-1280", width: 1280, height: 800 },
];

async function clickResultTab(page, label) {
  await page.evaluate((target) => {
    const button = [...document.querySelectorAll(".results-section-nav button")]
      .find((node) => node.textContent?.toLowerCase().includes(String(target).toLowerCase()));
    if (button instanceof HTMLButtonElement) button.click();
  }, label);
}

async function assistantSnapshot(page) {
  return page.evaluate(() => {
    const body = document.body.innerText;
    const assistant = document.querySelector("[data-v230-reportability-assistant='true']");
    const checklist = document.querySelector("[data-v230-reportability-checklist='true']");
    const items = [...document.querySelectorAll("[data-v230-reportability-item]")];
    const snippets = document.querySelector("[data-v230-report-snippets='true']");
    const rect = document.documentElement.getBoundingClientRect();
    return {
      hasAssistant: Boolean(assistant),
      hasChecklist: Boolean(checklist),
      itemIds: items.map((item) => item.getAttribute("data-v230-reportability-item") ?? ""),
      hasIssueLane: Boolean(assistant?.querySelector(".reportability-lane.issue")),
      hasReviewLane: Boolean(assistant?.querySelector(".reportability-lane.review")),
      hasReadyLane: Boolean(assistant?.querySelector(".reportability-lane.ready")),
      hasUnavailableLane: Boolean(assistant?.querySelector(".reportability-lane.unavailable")),
      hasWhatValueSays: body.includes("What the value says"),
      hasWhyItMatters: body.includes("Why it matters"),
      hasInspectNext: body.includes("What to inspect next"),
      hasReportWording: body.includes("Report wording"),
      hasCopySnippets: body.includes("Copy report snippets"),
      hasThresholdCaveat: body.includes("Threshold colors are guidance, not universal pass/fail rules"),
      hasValueSpecificEvidence: /0\.\d{3,4}|12 observations|5 path/.test(body),
      hasReportSnippets: Boolean(snippets?.textContent?.includes("Model setup") || snippets?.textContent?.includes("Structural model")),
      noR2Mojibake: !body.includes("RÃ‚Â²") && !body.includes("RÃƒ") && !body.includes("Ãƒâ€š") && !body.includes("ÃƒÆ’"),
      noHorizontalOverflow: document.documentElement.scrollWidth <= Math.ceil(rect.width) + 4,
    };
  });
}

async function runViewport(viewport, index) {
  const port = 57300 + index;
  return withPreviewPage({
    port,
    viewport: { width: viewport.width, height: viewport.height },
    run: async ({ page, errors }) => {
      await page.evaluate(() => {
        window.__QUICKPLS_SMOKE__?.loadDiagramFixture?.("mediation");
        window.__QUICKPLS_SMOKE__?.addCompletedRun?.();
        window.__QUICKPLS_SMOKE__?.setView?.("runs");
      });
      await page.waitForSelector("[data-v230-reportability-assistant='true']", { timeout: 10_000 });
      const overview = await assistantSnapshot(page);
      const overviewShot = path.join(SCREEN_DIR, `${viewport.name}-overview-assistant.png`);
      await page.screenshot({ path: overviewShot, fullPage: false });

      await clickResultTab(page, "Interpretation");
      await page.waitForSelector(".interpretation-workspace [data-v230-reportability-assistant='true']", { timeout: 10_000 });
      const interpretation = await assistantSnapshot(page);
      const interpretationShot = path.join(SCREEN_DIR, `${viewport.name}-interpretation-checklist.png`);
      await page.screenshot({ path: interpretationShot, fullPage: false });

      await clickResultTab(page, "Structural");
      await page.waitForSelector("[data-result-table-title='Path coefficients']", { timeout: 10_000 });
      const structuralText = await page.locator("body").innerText();

      return {
        viewport: viewport.name,
        overview,
        interpretation,
        structuralHasReportabilityNavigation: structuralText.includes("Path coefficients") && structuralText.includes("Structural findings"),
        screenshots: [overviewShot, interpretationShot],
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
  assistant_present: runs.every((run) => run.overview.hasAssistant && run.interpretation.hasAssistant),
  checklist_present: runs.every((run) => run.overview.hasChecklist),
  lanes_present: runs.every((run) => run.overview.hasIssueLane && run.overview.hasReviewLane && run.overview.hasReadyLane && run.overview.hasUnavailableLane),
  value_specific_sections_present: runs.every((run) => run.overview.hasWhatValueSays && run.overview.hasWhyItMatters && run.overview.hasInspectNext && run.overview.hasReportWording),
  copy_report_snippets_present: runs.every((run) => run.overview.hasCopySnippets),
  threshold_guidance_caveat_present: runs.every((run) => run.overview.hasThresholdCaveat),
  actual_run_values_present: runs.every((run) => run.overview.hasValueSpecificEvidence),
  report_snippets_present: runs.every((run) => run.interpretation.hasReportSnippets),
  canonical_items_present: runs.every((run) => ["indicator_reliability", "discriminant_validity", "structural_paths", "inference", "warnings"].every((id) => run.overview.itemIds.includes(id))),
  structural_navigation_still_works: runs.every((run) => run.structuralHasReportabilityNavigation),
  no_console_errors: runs.every((run) => run.consoleErrors.length === 0),
  no_r2_mojibake: runs.every((run) => run.overview.noR2Mojibake && run.interpretation.noR2Mojibake),
  no_document_horizontal_overflow: runs.every((run) => run.overview.noHorizontalOverflow && run.interpretation.noHorizontalOverflow),
};

const issues = Object.entries(checks)
  .filter(([, passed]) => !passed)
  .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));

const payload = {
  passed: issues.length === 0,
  milestone: "v2_30_0_interpretation_reportability_assistant",
  generatedAt: new Date().toISOString(),
  browserPlugin: "absent; Playwright smoke harness used",
  checks,
  issues,
  runs,
};

await writeJson(OUTPUT, payload);
await fs.writeFile(path.join(SCREEN_DIR, "README.txt"), "v2.30 reportability assistant smoke screenshots.\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(payload, null, 2));
