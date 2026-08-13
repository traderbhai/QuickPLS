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

const ARTIFACTS = path.join(RESULTS, "screens", "v252", "launcher-support-shell");
const OUTPUT = path.join(RESULTS, "v252_launcher_support_shell_smoke.json");
const PORT = 53252;

await ensureDir(ARTIFACTS);

async function shellState(page) {
  return page.evaluate(() => {
    const workflow = document.querySelector("[data-workflow-scope='primary-research-workflow']");
    const coach = document.querySelector(".workspace-coach");
    const host = document.querySelector(".page-host");
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? null,
      hasWorkflow: Boolean(workflow),
      workflowCount: workflow?.getAttribute("data-workflow-count") ?? null,
      workflowText: workflow?.textContent ?? "",
      hasCoach: Boolean(coach),
      hostClass: host?.className ?? "",
    };
  });
}

async function navigate(page, view, screenshotName) {
  await page.locator(`[data-nav-view="${view}"]`).click();
  await page.waitForTimeout(150);
  const state = await shellState(page);
  const snapshot = await collectV2ShellSnapshot(page);
  await page.screenshot({ path: path.join(ARTIFACTS, screenshotName), fullPage: true });
  return { state, snapshot };
}

const result = await withPreviewPage({
  port: PORT,
  run: async ({ page, errors }) => {
    const homeState = await shellState(page);
    const homeSnapshot = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "01_home_support_shell.png"), fullPage: true });

    const data = await navigate(page, "data", "02_data_primary_workflow.png");
    const trust = await navigate(page, "trust", "03_trust_support_shell.png");
    const settings = await navigate(page, "settings", "04_settings_support_shell.png");
    const model = await navigate(page, "models", "05_model_primary_workflow.png");

    const dataShell = evaluateV2ShellIntegrity(data.snapshot, "v2.5.2 launcher/support shell");
    const checklist = {
      home_is_launcher_without_primary_workflow: homeSnapshot.view === "welcome" && !homeState.hasWorkflow && !homeState.hasCoach && homeState.hostClass.includes("support-shell"),
      data_keeps_primary_workflow: data.snapshot.view === "data" && data.state.hasWorkflow && data.state.hasCoach && data.state.workflowCount === "6" && data.state.hostClass.includes("has-workflow-band"),
      model_keeps_primary_workflow_band: model.snapshot.view === "models" && model.state.hasWorkflow && model.state.hasCoach,
      trust_is_support_without_primary_workflow: trust.snapshot.view === "trust" && !trust.state.hasWorkflow && !trust.state.hasCoach && trust.state.hostClass.includes("support-shell"),
      settings_is_support_without_primary_workflow: settings.snapshot.view === "settings" && !settings.state.hasWorkflow && !settings.state.hasCoach && settings.state.hostClass.includes("support-shell"),
      support_shell_keeps_left_rail_utilities: settings.snapshot.rail.includes("Trust") && settings.snapshot.rail.includes("Settings"),
      primary_workflow_integrity_on_data: Object.values(dataShell).every(Boolean),
      no_console_errors: errors.length === 0,
    };
    const issues = issuesFromChecklist(checklist);
    return {
      schema_version: 1,
      target: "QuickPLS v2.5.2 launcher/support shell separation smoke",
      passed: issues.length === 0,
      generated_at: new Date().toISOString(),
      checklist,
      states: {
        home: homeState,
        data: data.state,
        model: model.state,
        trust: trust.state,
        settings: settings.state,
      },
      snapshots: {
        home: { ...homeSnapshot, body: undefined },
        data: { ...data.snapshot, body: undefined },
        model: { ...model.snapshot, body: undefined },
        trust: { ...trust.snapshot, body: undefined },
        settings: { ...settings.snapshot, body: undefined },
      },
      issues,
      errors,
      screenshots_dir: ARTIFACTS,
    };
  },
});

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
