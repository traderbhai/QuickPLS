import fs from "node:fs/promises";
import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const TARGET = "v2_7_0_visual_issue_register";
const ARTIFACTS = path.join(RESULTS, "screens", "v270", "visual-issue-register");
const OUTPUT = path.join(RESULTS, "v270_visual_issue_register_smoke.json");
const REGISTER = path.join(RESULTS, "v270_visual_issue_register.json");

await ensureDir(ARTIFACTS);

const views = [
  { view: "welcome", selector: "[data-nav-view='welcome']", family: "support" },
  { view: "data", selector: "[data-nav-view='data']", family: "workflow" },
  { view: "models", selector: "[data-nav-view='models']", family: "workflow" },
  { view: "analyses", selector: "[data-nav-view='analyses']", family: "workflow" },
  { view: "run", selector: "[data-nav-view='run']", family: "workflow" },
  { view: "runs", selector: "[data-nav-view='runs']", family: "workflow" },
  { view: "reports", selector: "[data-nav-view='reports']", family: "workflow" },
  { view: "trust", selector: "[data-nav-view='trust']", family: "support" },
  { view: "settings", selector: "[data-nav-view='settings']", family: "support" },
];

async function collectViewState(page, target, screenshotDir) {
  await page.locator(target.selector).click();
  await page.waitForTimeout(200);
  const shell = await collectV2ShellSnapshot(page);
  const state = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    const pageHost = document.querySelector(".page-host");
    const header = document.querySelector(".workspace-page h1, .home-header h1, .qpls2-page-shell h1");
    const cards = [...document.querySelectorAll(".q2-card, .home-action-card, .summary-card, .trust-card")];
    const buttons = [...document.querySelectorAll("button")].map((button) => ({
      label: button.textContent?.replace(/\s+/g, " ").trim() ?? "",
      disabled: button.disabled,
      aria: button.getAttribute("aria-label") ?? "",
      title: button.getAttribute("title") ?? "",
      describedBy: button.getAttribute("aria-describedby") ?? "",
    }));
    return {
      header: header?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      visibleText: document.body.innerText,
      documentOverflowX: root.scrollWidth - root.clientWidth,
      pageHostOverflowX: pageHost ? pageHost.scrollWidth - pageHost.clientWidth : 0,
      cardCount: cards.length,
      overlyTallCards: cards.filter((card) => card.getBoundingClientRect().height > 260).length,
      disabledWithoutReason: buttons.filter((button) => button.disabled && !button.aria && !button.title && !button.describedBy && !button.label).length,
      genericTodoText: /TODO|placeholder only|coming soon/i.test(document.body.innerText),
      mojibake: /RÃ|Ãƒ|ï¿½/.test(document.body.innerText),
      smartplsEquivalenceClaim: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(document.body.innerText),
      normalUserValidationFixtureText: /Validation fixture/i.test(document.body.innerText),
    };
  });
  await page.screenshot({ path: path.join(screenshotDir, `${target.family}-${target.view}.png`), fullPage: true });
  return { ...target, state: { ...shell, ...state } };
}

async function captureViewport({ viewport, port, label }) {
  const dir = path.join(ARTIFACTS, label);
  await ensureDir(dir);
  return withPreviewPage({
    port,
    viewport,
    run: async ({ page, errors }) => {
      const states = [];
      for (const target of views) {
        states.push(await collectViewState(page, target, dir));
      }
      const checklist = {
        all_primary_views_render_headers: states.every((item) => item.state.header || item.state.body.length > 100),
        no_horizontal_document_overflow: states.every((item) => Math.max(item.state.documentOverflowX, item.state.pageHostOverflowX) <= 2),
        no_mojibake: states.every((item) => !item.state.mojibake && !item.state.hasR2Mojibake),
        no_smartpls_equivalence_claim: states.every((item) => !item.state.smartplsEquivalenceClaim && !item.state.hasSmartPlsEquivalence),
        no_user_facing_validation_fixture_text: states.every((item) => !item.state.normalUserValidationFixtureText),
        disabled_buttons_are_described: states.every((item) => item.state.disabledWithoutReason === 0),
        no_placeholder_copy: states.every((item) => !item.state.genericTodoText),
        no_console_errors: errors.length === 0,
      };
      return {
        viewport,
        screenshot_dir: dir,
        states,
        checklist,
        issues: issuesFromChecklist(checklist),
        errors,
      };
    },
  });
}

const viewports = [
  { label: "1440x900", viewport: { width: 1440, height: 900 }, port: 53270 },
  { label: "1280x800", viewport: { width: 1280, height: 800 }, port: 53271 },
];

const runs = [];
for (const config of viewports) {
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

const register = {
  schema_version: 1,
  milestone: TARGET,
  generated_at: new Date().toISOString(),
  source: "automated v2.7 rendered shell smoke",
  viewports: runs.map((run) => `${run.viewport.width}x${run.viewport.height}`),
  open_issues: issues,
  next_review_focus: issues.length
    ? "Resolve failed checks before the next QuickPLS 2.x UI milestone."
    : "No launch-blocking rendered shell issues detected in the v2.7 smoke set.",
};

const result = {
  schema_version: 1,
  target: "QuickPLS v2.7.0 visual issue register smoke",
  passed: issues.length === 0,
  generated_at: new Date().toISOString(),
  runs,
  issues,
  screenshots_dir: ARTIFACTS,
  issue_register: REGISTER,
};

await writeJson(OUTPUT, result);
await fs.writeFile(REGISTER, JSON.stringify(register, null, 2));
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
