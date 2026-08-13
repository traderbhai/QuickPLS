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

const ARTIFACTS = path.join(RESULTS, "screens", "v251", "workflow-navigation");
const OUTPUT = path.join(RESULTS, "v251_workflow_navigation_smoke.json");
const PORT = 53251;

await ensureDir(ARTIFACTS);

const result = await withPreviewPage({
  port: PORT,
  run: async ({ page, errors }) => {
    const snapshot = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "01_workflow_home.png"), fullPage: true });

    const navigation = await page.evaluate(() => {
      const railSections = [...document.querySelectorAll("[data-nav-section]")].map((section) => ({
        label: section.getAttribute("data-nav-section"),
        views: [...section.querySelectorAll("[data-nav-view]")].map((button) => button.getAttribute("data-nav-view")),
      }));
      const workflow = document.querySelector("[data-workflow-scope='primary-research-workflow']");
      const workflowViews = workflow
        ? [...workflow.querySelectorAll("[data-workflow-view]")].map((button) => button.getAttribute("data-workflow-view"))
        : [];
      return {
        railSections,
        workflowScope: workflow?.getAttribute("data-workflow-scope") ?? null,
        workflowCount: workflow?.getAttribute("data-workflow-count") ?? null,
        workflowViews,
        workflowLabels: workflow
          ? [...workflow.querySelectorAll("[data-workflow-label]")].map((button) => button.getAttribute("data-workflow-label"))
          : [],
        workflowText: workflow?.textContent ?? "",
      };
    });

    await page.locator('[data-nav-view="settings"]').click();
    await page.waitForTimeout(150);
    const settingsSnapshot = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "02_support_settings_not_workflow.png"), fullPage: true });

    const workflowViews = navigation.workflowViews;
    const expectedWorkflow = ["data", "models", "analyses", "run", "runs", "reports"];
    const workflowText = navigation.workflowText.toLowerCase();
    const checklist = {
      ...evaluateV2ShellIntegrity(snapshot, "v2.5.1 workflow navigation parity"),
      workflow_scope_is_primary: navigation.workflowScope === "primary-research-workflow",
      workflow_step_count_is_six: navigation.workflowCount === "6" && workflowViews.length === 6,
      workflow_order_matches_research_flow: JSON.stringify(workflowViews) === JSON.stringify(expectedWorkflow),
      workflow_excludes_support_utilities: !workflowViews.includes("trust") && !workflowViews.includes("settings"),
      workflow_excludes_home_launcher: !workflowViews.includes("welcome"),
      workflow_has_visible_label: workflowText.includes("workflow"),
      rail_still_contains_support_utilities: navigation.railSections.some((section) => section.label === "Support" && JSON.stringify(section.views) === JSON.stringify(["trust", "settings"])),
      support_route_does_not_change_workflow_scope: settingsSnapshot.view === "settings",
      no_console_errors: errors.length === 0,
    };
    const issues = issuesFromChecklist(checklist);
    return {
      schema_version: 1,
      target: "QuickPLS v2.5.1 workflow navigation parity smoke",
      passed: issues.length === 0,
      generated_at: new Date().toISOString(),
      checklist,
      navigation,
      snapshot: { ...snapshot, body: undefined },
      after_settings_click: { ...settingsSnapshot, body: undefined },
      issues,
      errors,
      screenshots_dir: ARTIFACTS,
    };
  },
});

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
