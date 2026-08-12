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

const ARTIFACTS = path.join(RESULTS, "screens", "v253", "support-shell");
const OUTPUT = path.join(RESULTS, "v253_support_shell_smoke.json");
const PORT = 53253;

await ensureDir(ARTIFACTS);

async function supportState(page) {
  return page.evaluate(() => {
    const bar = document.querySelector("[data-support-shell='launcher-support']");
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? null,
      hasSupportBar: Boolean(bar),
      supportViews: bar ? [...bar.querySelectorAll("[data-support-view]")].map((button) => button.getAttribute("data-support-view")) : [],
      activeSupportView: bar?.querySelector("[aria-current='page']")?.getAttribute("data-support-view") ?? null,
      hasWorkflow: Boolean(document.querySelector("[data-workflow-scope='primary-research-workflow']")),
      hasCoach: Boolean(document.querySelector(".workspace-coach")),
    };
  });
}

async function clickNav(page, view, screenshotName) {
  await page.locator(`[data-nav-view="${view}"]`).click();
  await page.waitForTimeout(150);
  await page.screenshot({ path: path.join(ARTIFACTS, screenshotName), fullPage: true });
  return { snapshot: await collectV2ShellSnapshot(page), state: await supportState(page) };
}

async function clickSupport(page, view, screenshotName) {
  await page.locator(`[data-support-view="${view}"]`).click();
  await page.waitForTimeout(150);
  await page.screenshot({ path: path.join(ARTIFACTS, screenshotName), fullPage: true });
  return { snapshot: await collectV2ShellSnapshot(page), state: await supportState(page) };
}

const result = await withPreviewPage({
  port: PORT,
  run: async ({ page, errors }) => {
    const homeState = await supportState(page);
    const homeSnapshot = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "01_home_support_bar.png"), fullPage: true });

    const trust = await clickSupport(page, "trust", "02_support_bar_trust.png");
    const settings = await clickSupport(page, "settings", "03_support_bar_settings.png");
    const backHome = await clickSupport(page, "welcome", "04_support_bar_home_return.png");
    const data = await clickNav(page, "data", "05_data_no_support_bar.png");
    const model = await clickNav(page, "models", "06_model_no_support_bar.png");
    const dataShell = evaluateV2ShellIntegrity(data.snapshot, "v2.5.3 support utility shell");

    const expectedSupportViews = ["welcome", "trust", "settings"];
    const checklist = {
      home_support_bar_present: homeSnapshot.view === "welcome" && homeState.hasSupportBar && JSON.stringify(homeState.supportViews) === JSON.stringify(expectedSupportViews) && homeState.activeSupportView === "welcome",
      trust_support_navigation_works: trust.snapshot.view === "trust" && trust.state.hasSupportBar && trust.state.activeSupportView === "trust" && !trust.state.hasWorkflow && !trust.state.hasCoach,
      settings_support_navigation_works: settings.snapshot.view === "settings" && settings.state.hasSupportBar && settings.state.activeSupportView === "settings" && !settings.state.hasWorkflow && !settings.state.hasCoach,
      home_return_support_navigation_works: backHome.snapshot.view === "welcome" && backHome.state.hasSupportBar && backHome.state.activeSupportView === "welcome",
      data_excludes_support_bar_and_keeps_workflow: data.snapshot.view === "data" && !data.state.hasSupportBar && data.state.hasWorkflow && data.state.hasCoach,
      model_excludes_support_bar_and_keeps_workflow: model.snapshot.view === "models" && !model.state.hasSupportBar && model.state.hasWorkflow && model.state.hasCoach,
      data_shell_integrity_preserved: Object.values(dataShell).every(Boolean),
      no_console_errors: errors.length === 0,
    };
    const issues = issuesFromChecklist(checklist);
    return {
      schema_version: 1,
      target: "QuickPLS v2.5.3 support utility shell smoke",
      passed: issues.length === 0,
      generated_at: new Date().toISOString(),
      checklist,
      states: {
        home: homeState,
        trust: trust.state,
        settings: settings.state,
        back_home: backHome.state,
        data: data.state,
        model: model.state,
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
