import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const ARTIFACTS = path.join(RESULTS, "screens", "v260", "launch-quality");
const OUTPUT = path.join(RESULTS, "v260_launch_quality_smoke.json");

await ensureDir(ARTIFACTS);

const workflowViews = ["data", "models", "analyses", "run", "runs", "reports"];
const supportViews = [
  { view: "welcome", selector: "[data-support-view='welcome']" },
  { view: "trust", selector: "[data-support-view='trust']" },
  { view: "settings", selector: "[data-support-view='settings']" },
];

async function collectLaunchState(page) {
  const shell = await collectV2ShellSnapshot(page);
  const layout = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    const pageHost = document.querySelector(".page-host");
    const workspacePage = document.querySelector(".workspace-page");
    const supportFrame = document.querySelector(".support-utility-frame");
    const visibleHeader =
      document.querySelector(".workspace-page h1, .home-header h1, .qpls2-page-shell h1")?.textContent?.trim() ?? "";
    const buttons = [...document.querySelectorAll("button")].map((button) => ({
      label: button.textContent?.replace(/\s+/g, " ").trim() ?? "",
      disabled: button.disabled,
      aria: button.getAttribute("aria-label") ?? "",
      title: button.getAttribute("title") ?? "",
    }));
    return {
      visibleHeader,
      hasWorkspacePage: Boolean(workspacePage),
      hasSupportFrame: Boolean(supportFrame),
      documentOverflowX: root.scrollWidth - root.clientWidth,
      pageHostOverflowX: pageHost ? pageHost.scrollWidth - pageHost.clientWidth : 0,
      hasHorizontalScrollbar: root.scrollWidth > root.clientWidth + 2 || (pageHost ? pageHost.scrollWidth > pageHost.clientWidth + 2 : false),
      hasMojibake: document.body.innerText.includes("RÃ‚Â²") || document.body.innerText.includes("RÃƒ") || document.body.innerText.includes("Ãƒâ€š"),
      hasSmartPlsEquivalence: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(document.body.innerText),
      disabledWithoutDescription: buttons.filter((button) => button.disabled && !button.aria && !button.title && !button.label).length,
    };
  });
  return { ...shell, ...layout };
}

async function openView(page, target) {
  if (target.selector) {
    await page.locator(target.selector).click();
  } else {
    await page.locator(`[data-nav-view='${target.view}']`).click();
  }
  await page.waitForTimeout(200);
}

async function captureViewport({ viewport, port, label }) {
  const dir = path.join(ARTIFACTS, label);
  await ensureDir(dir);
  return withPreviewPage({
    port,
    viewport,
    run: async ({ page, errors }) => {
      const states = [];

      for (const target of supportViews) {
        await openView(page, target);
        const state = await collectLaunchState(page);
        states.push({ view: target.view, type: "support", state });
        await page.screenshot({ path: path.join(dir, `support-${target.view}.png`), fullPage: true });
      }

      for (const view of workflowViews) {
        await openView(page, { view });
        const state = await collectLaunchState(page);
        states.push({ view, type: "workflow", state });
        await page.screenshot({ path: path.join(dir, `workflow-${view}.png`), fullPage: true });
      }

      const supportStates = states.filter((item) => item.type === "support");
      const workflowStates = states.filter((item) => item.type === "workflow");
      const checklist = {
        all_views_render_headers: states.every((item) => item.state.visibleHeader.length > 0 || item.state.body.length > 100),
        support_views_use_support_frame: supportStates.every((item) => item.state.hasSupportFrame),
        workflow_views_hide_support_frame: workflowStates.every((item) => !item.state.hasSupportFrame),
        workflow_views_keep_primary_workflow: workflowStates.every((item) => item.state.workflow?.length >= 6),
        no_document_horizontal_overflow: states.every((item) => Math.max(item.state.documentOverflowX ?? 0, item.state.pageHostOverflowX ?? 0) <= 2),
        no_mojibake: states.every((item) => !item.state.hasMojibake && !item.state.hasR2Mojibake),
        no_smartpls_equivalence_claim: states.every((item) => !item.state.hasSmartPlsEquivalence && !item.state.hasSmartPlsEquivalenceClaim),
        disabled_buttons_are_described: states.every((item) => item.state.disabledWithoutDescription === 0),
        no_console_errors: errors.length === 0,
      };

      return {
        viewport,
        screenshot_dir: dir,
        checklist,
        states,
        issues: issuesFromChecklist(checklist),
        errors,
      };
    },
  });
}

const viewports = [
  { label: "1440x900", viewport: { width: 1440, height: 900 }, port: 53260 },
  { label: "1280x800", viewport: { width: 1280, height: 800 }, port: 53261 },
];

const runs = [];
for (const config of viewports) {
  runs.push(await captureViewport(config));
}

const issues = runs.flatMap((run) => run.issues.map((issue) => ({ ...issue, viewport: `${run.viewport.width}x${run.viewport.height}` })));
const errors = runs.flatMap((run) => run.errors.map((error) => ({ viewport: `${run.viewport.width}x${run.viewport.height}`, error })));
const result = {
  schema_version: 1,
  target: "QuickPLS v2.6.0 launch quality visual consolidation smoke",
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
