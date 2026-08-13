import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  evaluateV2ShellIntegrity,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const ARTIFACTS = path.join(RESULTS, "screens", "v250", "navigation-hierarchy");
const OUTPUT = path.join(RESULTS, "v250_navigation_hierarchy_smoke.json");
const PORT = 53250;

await ensureDir(ARTIFACTS);

const result = await withPreviewPage({
  port: PORT,
  run: async ({ page, errors }) => {
    const snapshot = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "01_navigation_hierarchy_home.png"), fullPage: true });

    const nav = await page.locator(".nav-rail").evaluate((rail) => {
      const sections = [...rail.querySelectorAll("[data-nav-section]")].map((section) => ({
        label: section.getAttribute("data-nav-section"),
        views: [...section.querySelectorAll("[data-nav-view]")].map((button) => button.getAttribute("data-nav-view")),
        labels: [...section.querySelectorAll("[data-nav-view] span")].map((span) => span.textContent?.trim()),
      }));
      const active = rail.querySelector(".nav-item.active")?.getAttribute("data-nav-view");
      return { sections, active };
    });

    await page.locator('[data-nav-view="trust"]').click();
    await page.waitForTimeout(150);
    const trustSnapshot = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "02_navigation_hierarchy_trust.png"), fullPage: true });

    const workflowSection = nav.sections.find((section) => section.label === "Research workflow");
    const supportSection = nav.sections.find((section) => section.label === "Support");
    const checklist = {
      ...evaluateV2ShellIntegrity(snapshot, "v2.5.0 navigation hierarchy"),
      research_workflow_section_exists: Boolean(workflowSection),
      support_section_exists: Boolean(supportSection),
      primary_workflow_order_is_clear: JSON.stringify(workflowSection?.views) === JSON.stringify(["welcome", "data", "models", "analyses", "run", "runs", "reports"]),
      support_utilities_are_separated: JSON.stringify(supportSection?.views) === JSON.stringify(["trust", "settings"]),
      trust_navigation_works: trustSnapshot.view === "trust",
      no_console_errors: errors.length === 0,
    };
    const issues = issuesFromChecklist(checklist);
    return {
      schema_version: 1,
      target: "QuickPLS v2.5.0 navigation hierarchy smoke",
      passed: issues.length === 0,
      generated_at: new Date().toISOString(),
      checklist,
      nav,
      snapshot: { ...snapshot, body: undefined },
      after_trust_click: { ...trustSnapshot, body: undefined },
      issues,
      errors,
      screenshots_dir: ARTIFACTS,
    };
  },
});

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
