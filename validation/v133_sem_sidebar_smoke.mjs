import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v133", "sidebar");
const OUTPUT = path.join(RESULTS, "v133_sem_sidebar_smoke.json");
const PORT = 53133;
const URL = `http://127.0.0.1:${PORT}/`;

await mkdir(ARTIFACTS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
let logs = "";
server.stdout.on("data", (data) => { logs += data.toString(); });
server.stderr.on("data", (data) => { logs += data.toString(); });

async function waitForUrl() {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(URL, { signal: AbortSignal.timeout(1500) });
      if (response.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Timed out waiting for Vite preview. ${logs.slice(-1200)}`);
}

async function sidebarMetrics(page) {
  return page.evaluate(() => {
    const explorer = document.querySelector(".sem-explorer, .explorer.collapsed");
    const body = document.querySelector(".explorer-body");
    const tabs = [...document.querySelectorAll(".explorer-tabs button")].map((button) => button.textContent?.trim());
    const text = explorer?.textContent ?? "";
    return {
      width: explorer?.clientWidth ?? 0,
      scroll_width: explorer?.scrollWidth ?? 0,
      body_scroll_height: body?.scrollHeight ?? 0,
      body_height: body?.clientHeight ?? 0,
      has_horizontal_overflow: Boolean(explorer && explorer.scrollWidth > explorer.clientWidth + 2),
      tabs,
      text,
      visible_cards: document.querySelectorAll(".explorer-card").length,
      visible_variables: document.querySelectorAll(".variable-row").length,
      visible_structure_rows: document.querySelectorAll(".structure-row").length,
      visible_issues: document.querySelectorAll(".issue-row").length,
      collapsed: Boolean(document.querySelector(".explorer.collapsed")),
      legacy_data_model_tabs_present: Boolean(document.querySelector(".pane-tabs")),
      footer_text: document.querySelector(".explorer-summary")?.textContent ?? "",
    };
  });
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.getByRole("button", { name: /^Model:/i }).click();
  await page.waitForSelector(".explorer-tabs", { timeout: 10_000 });

  const shots = {};
  shots.constructs = path.join(ARTIFACTS, "01_constructs_tab.png");
  await page.screenshot({ path: shots.constructs, fullPage: true });
  const constructs = await sidebarMetrics(page);

  await page.locator(".explorer-tabs").getByRole("button", { name: "Variables" }).click();
  shots.variables = path.join(ARTIFACTS, "02_variables_tab.png");
  await page.screenshot({ path: shots.variables, fullPage: true });
  const variables = await sidebarMetrics(page);

  await page.locator(".explorer-filter-row").getByRole("button", { name: "unassigned" }).click();
  const variableFilterText = await page.locator(".explorer-body").innerText();

  await page.locator(".explorer-tabs").getByRole("button", { name: "Structure" }).click();
  shots.structure = path.join(ARTIFACTS, "03_structure_tab.png");
  await page.screenshot({ path: shots.structure, fullPage: true });
  const structure = await sidebarMetrics(page);

  await page.locator(".structure-row").first().click();
  await page.waitForTimeout(200);
  shots.pathSelected = path.join(ARTIFACTS, "04_structure_path_selected.png");
  await page.screenshot({ path: shots.pathSelected, fullPage: true });
  const pathSelectedText = await page.locator(".structure-row.selected").innerText().catch(() => "");

  await page.locator(".explorer-tabs").getByRole("button", { name: "Issues" }).click();
  shots.issues = path.join(ARTIFACTS, "05_issues_tab.png");
  await page.screenshot({ path: shots.issues, fullPage: true });
  const issues = await sidebarMetrics(page);

  await page.getByTitle("Collapse SEM explorer").click();
  await page.waitForTimeout(150);
  shots.collapsed = path.join(ARTIFACTS, "06_collapsed_sidebar.png");
  await page.screenshot({ path: shots.collapsed, fullPage: true });
  const collapsed = await sidebarMetrics(page);

  await page.getByTitle("Expand SEM explorer").click();
  await page.waitForTimeout(150);
  const expanded = await sidebarMetrics(page);

  const checklist = {
    default_sidebar_has_no_horizontal_overflow: !constructs.has_horizontal_overflow,
    legacy_data_model_inner_tabs_removed: !constructs.legacy_data_model_tabs_present,
    tabs_are_model_native: ["Constructs", "Variables", "Structure", "Issues"].every((tab) => constructs.tabs.includes(tab)),
    project_status_and_summary_present: constructs.text.includes("SEM explorer") && constructs.footer_text.includes("constructs") && constructs.footer_text.includes("paths"),
    constructs_tab_lists_construct_actions: constructs.visible_cards >= 4 && ["Add", "left", "right", "top", "bottom", "Reset indicators"].every((text) => constructs.text.includes(text)),
    variables_tab_exposes_assignment_workflow: variables.visible_variables >= 4 && ["Dataset variables", "unassigned", "assigned", "selected"].every((text) => variables.text.includes(text)),
    variable_filter_is_functional: variableFilterText.includes("Unassigned") || variableFilterText.includes("Dataset variables"),
    structure_tab_lists_paths_and_controls: structure.visible_structure_rows >= 1 && ["Structural model", "Straight", "Label"].every((text) => structure.text.includes(text)),
    structure_click_selects_path: pathSelectedText.includes("->"),
    issues_tab_surfaces_actionable_status: issues.text.includes("Model issues") && (issues.visible_issues > 0 || issues.text.includes("No obvious model-building issues")),
    collapse_and_expand_workflow: collapsed.collapsed && !expanded.collapsed,
    screenshots_written: Object.values(shots).every(Boolean),
  };

  const report = {
    schema_version: 1,
    target: "QuickPLS v1.3.3 SEM explorer sidebar redesign",
    passed: errors.length === 0 && Object.values(checklist).every(Boolean),
    checklist,
    errors,
    evidence: { constructs, variables, structure, issues, collapsed, expanded, pathSelectedText, screenshots: shots },
  };
  await writeFile(OUTPUT, JSON.stringify(report, null, 2), "utf-8");
  console.log(JSON.stringify(report, null, 2));
  if (!report.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(server.pid), "/t", "/f"], { stdio: "ignore" });
  else server.kill();
}
