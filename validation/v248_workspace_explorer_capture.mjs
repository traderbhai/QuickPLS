import path from "node:path";
import { chromium } from "playwright";

const endpoint = process.env.QUICKPLS_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const projectPath = path.resolve(process.env.QUICKPLS_CAPTURE_PROJECT_PATH ?? "");
const projectName = process.env.QUICKPLS_CAPTURE_PROJECT_NAME?.trim() || "Native Methods Acceptance";
const recentProjectsKey = "quickpls.native.recent-projects.v1";

if (!process.env.QUICKPLS_CAPTURE_PROJECT_PATH) {
  throw new Error("QUICKPLS_CAPTURE_PROJECT_PATH is required.");
}

const browser = await chromium.connectOverCDP(endpoint);
const context = browser.contexts()[0];
const page = context?.pages().find((candidate) => candidate.url().startsWith("http://tauri.localhost"))
  ?? context?.pages()[0];
if (!page) throw new Error("No QuickPLS WebView page is available.");

const priorRecentProjects = await page.evaluate((key) => window.localStorage.getItem(key), recentProjectsKey);

try {
  await page.evaluate(({ key, project }) => {
    let existing = [];
    try {
      const parsed = JSON.parse(window.localStorage.getItem(key) ?? "[]");
      if (Array.isArray(parsed)) existing = parsed;
    } catch {
      existing = [];
    }
    const normalizedPath = project.path.toLocaleLowerCase();
    window.localStorage.setItem(key, JSON.stringify([
      project,
      ...existing.filter((entry) => typeof entry?.path === "string" && entry.path.toLocaleLowerCase() !== normalizedPath),
    ].slice(0, 8)));
  }, {
    key: recentProjectsKey,
    project: { name: projectName, path: projectPath, openedAt: new Date().toISOString() },
  });

  await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.locator('.nd-app[data-surface="launcher"]').waitFor({ state: "visible", timeout: 15_000 });
  const recentRow = page.locator(".nd-recent-projects .nd-project-row").filter({ hasText: projectName });
  await recentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await recentRow.count() !== 1) throw new Error("The capture project was not shown as one recent-project row.");
  await recentRow.click();
  await page.locator(".nd-window-project").filter({ hasText: projectName }).waitFor({ state: "visible", timeout: 15_000 });

  const viewMenuTrigger = page.getByRole("menuitem", { name: "View", exact: true });
  await viewMenuTrigger.click();
  const popupId = await viewMenuTrigger.getAttribute("aria-controls");
  if (!popupId) throw new Error("The View menu did not expose its popup.");
  const viewMenu = page.locator(`#${popupId}`);
  await viewMenu.getByRole("menuitem", { name: "Project", exact: true }).click();

  const tree = page.getByRole("tree", { name: `${projectName} project contents`, exact: true });
  await tree.waitFor({ state: "visible", timeout: 15_000 });
  const models = tree.locator('.nd-project-treeitem[data-kind="model"]');
  const reports = tree.locator('.nd-project-treeitem[data-kind="report"]');
  if (await models.count() !== 2 || await reports.count() !== 1) {
    throw new Error(`Expected two editable models and one saved report, found ${await models.count()} and ${await reports.count()}.`);
  }
  await tree.locator('.nd-project-treeitem[data-kind="project"]').click();
  const selectedHeading = page.locator("#nd-explorer-detail-title");
  await selectedHeading.waitFor({ state: "visible", timeout: 5_000 });
  if ((await selectedHeading.textContent())?.trim() !== projectName) {
    throw new Error("The Project root did not synchronize tree selection and detail content.");
  }

  const state = await tree.evaluate((element) => ({
    labels: Array.from(element.querySelectorAll(".nd-tree-label")).map((node) => node.textContent?.trim() ?? ""),
    tabStops: element.querySelectorAll('[role="treeitem"][tabindex="0"]').length,
    selectedItems: element.querySelectorAll('[role="treeitem"][aria-selected="true"]').length,
    activeModels: element.querySelectorAll('[role="treeitem"].active-model').length,
  }));
  if (state.tabStops !== 1 || state.selectedItems !== 1 || state.activeModels !== 1) {
    throw new Error(`Workspace Explorer capture state is not accessible: ${JSON.stringify(state)}`);
  }
  console.log(JSON.stringify({ projectPath, projectName, selectedHeading: (await selectedHeading.textContent())?.trim(), models: await models.allTextContents(), reports: await reports.allTextContents(), ...state }, null, 2));
} finally {
  await page.evaluate(({ key, prior }) => {
    if (prior === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, prior);
  }, { key: recentProjectsKey, prior: priorRecentProjects }).catch(() => undefined);
  await browser.close();
}
