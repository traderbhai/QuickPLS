import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const ARTIFACTS = path.join(RESULTS, "screens", "v255", "support-viewport");
const OUTPUT = path.join(RESULTS, "v255_support_viewport_smoke.json");

await ensureDir(ARTIFACTS);

async function supportLayoutState(page) {
  return page.evaluate(() => {
    const frame = document.querySelector(".support-utility-frame");
    const bar = document.querySelector("[data-support-shell='launcher-support']");
    const workspace = document.querySelector(".workspace-page");
    const frameRect = frame?.getBoundingClientRect();
    const barRect = bar?.getBoundingClientRect();
    const workspaceRect = workspace?.getBoundingClientRect();
    const root = document.scrollingElement ?? document.documentElement;
    const pageHost = document.querySelector(".page-host");
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? null,
      hasSupportFrame: Boolean(frame),
      hasSupportBar: Boolean(bar),
      activeSupportView: bar?.querySelector("[aria-current='page']")?.getAttribute("data-support-view") ?? null,
      supportViewCount: bar?.querySelectorAll("[data-support-view]").length ?? 0,
      frameLeft: frameRect?.left ?? null,
      workspaceLeft: workspaceRect?.left ?? null,
      frameRight: frameRect?.right ?? null,
      workspaceRight: workspaceRect?.right ?? null,
      barWidth: barRect?.width ?? null,
      frameWidth: frameRect?.width ?? null,
      documentOverflowX: root.scrollWidth - root.clientWidth,
      pageHostOverflowX: pageHost ? pageHost.scrollWidth - pageHost.clientWidth : null,
      supportActionOverflow: [...(bar?.querySelectorAll("[data-support-view]") ?? [])].some((node) => node.scrollWidth > node.clientWidth + 1),
      workflowPresent: Boolean(document.querySelector("[data-workflow-scope='primary-research-workflow']")),
      coachPresent: Boolean(document.querySelector(".workspace-coach")),
    };
  });
}

function aligned(leftA, leftB, rightA, rightB) {
  return Number.isFinite(leftA) && Number.isFinite(leftB) && Number.isFinite(rightA) && Number.isFinite(rightB)
    && Math.abs(leftA - leftB) <= 2
    && Math.abs(rightA - rightB) <= 2;
}

async function captureViewport({ viewport, port, label }) {
  const dir = path.join(ARTIFACTS, label);
  await ensureDir(dir);
  return withPreviewPage({
    port,
    viewport,
    run: async ({ page, errors }) => {
      const home = await supportLayoutState(page);
      await page.screenshot({ path: path.join(dir, "01_home_support_aligned.png"), fullPage: true });

      await page.locator("[data-support-view='trust']").click();
      await page.waitForTimeout(150);
      const trust = await supportLayoutState(page);
      await page.screenshot({ path: path.join(dir, "02_trust_support_aligned.png"), fullPage: true });

      await page.locator("[data-support-view='settings']").click();
      await page.waitForTimeout(150);
      const settings = await supportLayoutState(page);
      await page.screenshot({ path: path.join(dir, "03_settings_support_aligned.png"), fullPage: true });

      await page.locator("[data-nav-view='data']").click();
      await page.waitForTimeout(150);
      const dataSnapshot = await collectV2ShellSnapshot(page);
      const dataState = await supportLayoutState(page);
      await page.screenshot({ path: path.join(dir, "04_data_no_support_frame.png"), fullPage: true });

      const supportStates = { home, trust, settings };
      const checklist = {
        home_support_aligned: aligned(home.frameLeft, home.workspaceLeft, home.frameRight, home.workspaceRight),
        trust_support_aligned: aligned(trust.frameLeft, trust.workspaceLeft, trust.frameRight, trust.workspaceRight),
        settings_support_aligned: aligned(settings.frameLeft, settings.workspaceLeft, settings.frameRight, settings.workspaceRight),
        support_views_present: Object.values(supportStates).every((state) => state.hasSupportFrame && state.hasSupportBar && state.supportViewCount === 3),
        active_support_states_correct: home.activeSupportView === "welcome" && trust.activeSupportView === "trust" && settings.activeSupportView === "settings",
        support_shell_has_no_workflow_controls: Object.values(supportStates).every((state) => !state.workflowPresent && !state.coachPresent),
        support_bar_fits_frame: Object.values(supportStates).every((state) => Number(state.barWidth) <= Number(state.frameWidth) + 1),
        no_support_action_text_overflow: Object.values(supportStates).every((state) => !state.supportActionOverflow),
        no_document_horizontal_overflow: [home, trust, settings, dataState].every((state) => Math.max(state.documentOverflowX ?? 0, state.pageHostOverflowX ?? 0) <= 2),
        data_keeps_workflow_without_support_shell: dataSnapshot.view === "data" && !dataState.hasSupportFrame && !dataState.hasSupportBar && dataState.workflowPresent && dataState.coachPresent,
        no_console_errors: errors.length === 0,
      };
      return {
        viewport,
        screenshot_dir: dir,
        checklist,
        states: { home, trust, settings, data: dataState },
        issues: issuesFromChecklist(checklist),
        errors,
      };
    },
  });
}

const viewports = [
  { label: "1440x900", viewport: { width: 1440, height: 900 }, port: 53255 },
  { label: "1280x800", viewport: { width: 1280, height: 800 }, port: 53256 },
];

const runs = [];
for (const config of viewports) {
  runs.push(await captureViewport(config));
}

const issues = runs.flatMap((run) => run.issues.map((issue) => ({ ...issue, viewport: `${run.viewport.width}x${run.viewport.height}` })));
const errors = runs.flatMap((run) => run.errors.map((error) => ({ viewport: `${run.viewport.width}x${run.viewport.height}`, error })));
const result = {
  schema_version: 1,
  target: "QuickPLS v2.5.5 support shell viewport alignment smoke",
  passed: issues.length === 0 && errors.length === 0,
  generated_at: new Date().toISOString(),
  runs,
  issues,
  errors,
  screenshots_dir: ARTIFACTS,
};

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
