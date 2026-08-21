#!/usr/bin/env node
/**
 * QuickPLS 2.52 packaged HOC journey.
 *
 * Run `execute` against one packaged process, close that process, then run
 * `reopen` against a newly launched packaged process. Both phases update one
 * compact report and one screenshot directory.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import {
  PACKAGED_TAURI_ORIGIN,
  connectToSingleQuickPlsPage,
  observeFunctionalOfflineRequests,
} from "./v247_cdp_package_helpers.mjs";
import {
  buildConstructs,
  canonicalIdentity,
  createEmptyModel,
  importFixture,
  waitForSurface,
} from "./general_sem_rank0_packaged_acceptance.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS_ROOT = path.join(ROOT, "validation", "results");
const FILE_DIALOG_HELPER = path.join(ROOT, "validation", "windows_native_owned_file_dialog.py");
const PROJECT_NAME = "QuickPLS 2.52 HOC packaged smoke";
const MODEL_NAME = "QuickPLS 2.52 HOC model";

function parseArgs(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) throw new Error(`Unexpected positional argument: ${token}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${token}`);
    values[token.slice(2)] = value;
    index += 1;
  }
  for (const key of ["phase", "endpoint", "evidence-dir", "project-path", "python"]) {
    if (!values[key]) throw new Error(`--${key} is required`);
  }
  if (!new Set(["execute", "reopen"]).has(values.phase)) {
    throw new Error("--phase must be execute or reopen");
  }
  return values;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

function compact(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

async function pathExists(file) {
  return fs.stat(file).then(() => true, () => false);
}

function createDialogHelper({ python, mode, target, allowedRoot, windowTitle, extension }) {
  const child = spawn(python, [
    FILE_DIALOG_HELPER,
    "--mode", mode,
    "--target", target,
    "--allowed-root", allowedRoot,
    "--window-title", windowTitle,
    "--timeout-seconds", "90",
    "--extension", extension,
  ], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  const events = [];
  let stdout = "";
  let stderr = "";
  let readyResolve;
  let completeResolve;
  let readySettled = false;
  let completeSettled = false;
  const ready = new Promise((resolve) => { readyResolve = resolve; });
  const completed = new Promise((resolve) => { completeResolve = resolve; });
  const settleReady = (event) => {
    if (!readySettled) { readySettled = true; readyResolve(event); }
  };
  const settleComplete = (event) => {
    if (!completeSettled) { completeSettled = true; completeResolve(event); }
  };
  const acceptLine = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      events.push(event);
      if (event.event === "ready") settleReady(event);
      if (event.event === "complete") {
        if (!event.passed) settleReady(event);
        settleComplete(event);
      }
    } catch (error) {
      const failure = { event: "complete", passed: false, phase: "jsonl", message: String(error), line };
      settleReady(failure);
      settleComplete(failure);
    }
  };
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdout += chunk;
    const lines = stdout.split(/\r?\n/);
    stdout = lines.pop() ?? "";
    lines.forEach(acceptLine);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.on("error", (error) => {
    const failure = { event: "complete", passed: false, phase: "spawn", message: error.message };
    settleReady(failure);
    settleComplete(failure);
  });
  child.on("close", (code, signal) => {
    if (stdout.trim()) acceptLine(stdout);
    const failure = { event: "complete", passed: false, phase: "exit", code, signal, stderr, events };
    settleReady(failure);
    settleComplete(failure);
  });
  return { ready, completed, stop: () => child.kill() };
}

async function createFixture(file, rowCount = 1_500) {
  const columns = ["capability_1", "capability_2", "resources_1", "resources_2", "performance_1", "performance_2"];
  const rows = Array.from({ length: rowCount }, (_, offset) => {
    const index = offset + 1;
    const capability = Math.sin(index * 0.17) + Math.cos(index * 0.071) * 0.28;
    const resources = Math.cos(index * 0.13) + Math.sin(index * 0.047) * 0.33;
    const disturbance = Math.sin(index * 0.91) * 0.08 + Math.cos(index * 0.53) * 0.04;
    const performance = 0.58 * capability + 0.42 * resources + disturbance;
    return [
      capability,
      capability * 0.91 + disturbance * 0.12,
      resources,
      resources * 0.89 - disturbance * 0.1,
      performance,
      performance * 0.93 + disturbance * 0.14,
    ].map((value) => value.toFixed(8)).join(",");
  });
  await fs.writeFile(file, `${columns.join(",")}\n${rows.join("\n")}\n`, "utf8");
  return { columns, rowCount };
}

async function createUnifiedProject(page, name) {
  await page.keyboard.press("Control+n");
  const dialog = page.getByRole("dialog", { name: "New Project", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByLabel("Project name", { exact: true }).fill(name);
  const note = compact(await dialog.locator(".nd-dialog-note").textContent());
  assert(/Canvas.*PLS-SEM.*CB-SEM.*Results/i.test(note), `New Project is not the unified scientific workflow: ${note}`);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await waitForSurface(page, "data");
}

async function openMenuItem(page, menuName, itemName) {
  const menubar = page.getByRole("menubar", { name: "Application menu" });
  const trigger = menubar.getByRole("menuitem", { name: menuName, exact: true });
  const popupId = await trigger.getAttribute("aria-controls");
  assert(popupId, `${menuName} menu trigger has no popup identity.`);
  await trigger.focus();
  await page.keyboard.press("ArrowDown");
  const menu = page.locator(`#${popupId}[role="menu"]`);
  await menu.waitFor({ state: "visible", timeout: 10_000 });
  const item = menu.getByRole("menuitem", { name: itemName });
  await item.waitFor({ state: "visible", timeout: 10_000 });
  assert(await item.isEnabled(), `${menuName} > ${String(itemName)} is disabled.`);
  await item.click();
}

async function capture(page, report, screenshotRoot, id, observation) {
  const file = path.join(screenshotRoot, `${id}.png`);
  await page.screenshot({ path: file, animations: "disabled" });
  const relative = path.relative(ROOT, file).split(path.sep).join("/");
  if (!report.screenshots.includes(relative)) report.screenshots.push(relative);
  report.observations = report.observations.filter((entry) => entry.id !== id);
  report.observations.push({ id, observation });
  return relative;
}

async function waitForCalculationTerminal(page, timeout = 180_000) {
  const state = page.locator(".nd-cbsem-v4-state");
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await page.locator(".nd-results-workspace").isVisible().catch(() => false)) return "completed";
    if (!await state.isVisible().catch(() => false)) {
      await page.waitForTimeout(250);
      continue;
    }
    const stateText = await state.textContent({ timeout: 1_000 }).catch(() => null);
    if (stateText === null) continue;
    const value = compact(stateText).toLowerCase();
    if (value === "completed") return value;
    if (value === "failed" || value === "cancelled") {
      const detail = compact(await page.locator(".nd-cbsem-v4-failure").textContent().catch(() => ""));
      throw new Error(`HOC calculation ${value}: ${detail || compact(await page.locator(".nd-cbsem-v4-monitor").textContent())}`);
    }
    await page.waitForTimeout(250);
  }
  throw new Error(`HOC calculation did not reach a terminal state: ${compact(await page.locator(".nd-cbsem-v4-monitor").textContent())}`);
}

async function assertHocResults(page) {
  const workspace = page.locator(".nd-results-workspace");
  await workspace.waitFor({ state: "visible", timeout: 300_000 });
  const tree = page.getByRole("tree", { name: "Available result sections" });
  await tree.waitFor({ state: "visible", timeout: 30_000 });
  const treeText = compact(await tree.textContent());
  assert(/Higher-Order Constructs/i.test(treeText), `Results lacks the Higher-Order Constructs group: ${treeText}`);
  const componentItem = tree.getByRole("treeitem", { name: /Component and structural estimates/i }).first();
  await componentItem.waitFor({ state: "visible", timeout: 30_000 });
  await componentItem.click();
  const table = page.locator('[data-canonical-table-id="general_sem_higher_order_targets"]');
  await table.waitFor({ state: "visible", timeout: 30_000 });
  const identity = await canonicalIdentity(page);
  assert(identity.documentId && identity.runId, `Canonical HOC identity is incomplete: ${JSON.stringify(identity)}`);
  return {
    identity,
    treeGroups: await tree.locator('[role="group"]').count(),
    selectedResult: compact(await page.locator(".nd-results-document").textContent()),
    tableId: await table.getAttribute("data-canonical-table-id"),
  };
}

async function createAndEditHoc(page, report, screenshotRoot) {
  await openMenuItem(page, "Model", /^Higher-Order Construct/);
  const createDialog = page.getByRole("dialog", { name: "Create Higher-Order Construct", exact: true });
  await createDialog.waitFor({ state: "visible", timeout: 10_000 });
  await createDialog.getByLabel("Name", { exact: true }).fill("Organizational Capability");
  await createDialog.getByRole("checkbox", { name: /Capability/ }).check();
  await createDialog.getByRole("checkbox", { name: /Resources/ }).check();
  const performance = createDialog.getByRole("checkbox", { name: /Performance/ });
  if (await performance.isChecked()) await performance.uncheck();
  const summary = compact(await createDialog.locator(".nd-hoc-summary").textContent());
  assert(/RR/.test(summary) && /Disjoint two-stage/i.test(summary), `HOC recommendation is not RR disjoint two-stage: ${summary}`);
  await capture(page, report, screenshotRoot, "01-hoc-dialog", "The compact native Create HOC dialog derives RR and recommends disjoint two-stage for two measurement-only dimensions.");
  await createDialog.getByRole("button", { name: "Create", exact: true }).click();
  await createDialog.waitFor({ state: "hidden", timeout: 10_000 });

  let hocNode = page.locator(".react-flow__node-latent").filter({ hasText: "Organizational Capability" });
  await hocNode.waitFor({ state: "visible", timeout: 10_000 });
  const constructId = await hocNode.getAttribute("data-id");
  assert(constructId, "The created HOC has no stable node identity.");
  await hocNode.dispatchEvent("click");
  await page.keyboard.press("Enter");
  let editDialog = page.getByRole("dialog", { name: "Edit Higher-Order Construct", exact: true });
  if (!await editDialog.waitFor({ state: "visible", timeout: 2_000 }).then(() => true, () => false)) {
    await openMenuItem(page, "Model", "Edit Higher-Order Construct…");
    editDialog = page.getByRole("dialog", { name: "Edit Higher-Order Construct", exact: true });
    await editDialog.waitFor({ state: "visible", timeout: 10_000 });
  }
  await editDialog.getByLabel("Name", { exact: true }).fill("Enterprise Capability");
  await editDialog.getByRole("button", { name: "Save", exact: true }).click();
  await editDialog.waitFor({ state: "hidden", timeout: 10_000 });
  hocNode = page.locator(`.react-flow__node-latent[data-id="${constructId}"]`);
  await hocNode.filter({ hasText: "Enterprise Capability" }).waitFor({ state: "visible", timeout: 10_000 });
  return constructId;
}

function scientificPaths(page) {
  return page.locator(
    '.react-flow__edge[data-id]:not([data-id^="measurement::"]):not([data-id^="hoc-membership::"])',
  );
}

async function createScientificPath(page, nodes, sourceIndex, targetIndex, expected) {
  await page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ }).click();
  await nodes.nth(sourceIndex).dispatchEvent("click");
  await nodes.nth(targetIndex).dispatchEvent("click");
  await scientificPaths(page).nth(expected - 1).waitFor({ state: "attached", timeout: 10_000 });
  assert(await scientificPaths(page).count() === expected, `Expected ${expected} persisted structural path.`);
}

async function executeJourney(page, args, report, screenshotRoot) {
  assert(!await pathExists(args.projectPath), `The execute project path must be new: ${args.projectPath}`);
  const fixturePath = path.join(args.evidenceDir, "hoc-input.csv");
  const fixture = await createFixture(fixturePath);
  await createUnifiedProject(page, PROJECT_NAME);
  await importFixture(page, args.python, fixturePath);
  await createEmptyModel(page, MODEL_NAME);
  const ordinaryNodes = await buildConstructs(page, [
    { name: "Capability", indicators: ["capability_1", "capability_2"] },
    { name: "Resources", indicators: ["resources_1", "resources_2"] },
    { name: "Performance", indicators: ["performance_1", "performance_2"] },
  ]);
  assert(await ordinaryNodes.count() === 3, "The HOC smoke did not create three ordinary constructs.");
  const hocId = await createAndEditHoc(page, report, screenshotRoot);
  const allNodes = page.locator(".react-flow__node-latent");
  assert(await allNodes.count() === 4, "HOC creation did not produce exactly four visible constructs.");
  await createScientificPath(page, allNodes, 3, 2, 1);
  await capture(page, report, screenshotRoot, "02-canvas", "Canvas shows the edited HOC, its two dimensions, and one authored HOC-to-outcome structural path; membership is a visual overlay only.");

  await page.keyboard.press("Control+R");
  const calculation = page.getByRole("dialog", { name: "Calculate", exact: true });
  await calculation.waitFor({ state: "visible", timeout: 15_000 });
  await calculation.locator("#nd-calculation-method-list").waitFor({ state: "visible", timeout: 15_000 });
  const methodOptions = calculation.locator('#nd-calculation-method-list [role="option"]');
  const methodCount = await methodOptions.count();
  assert(methodCount === 18, `Calculate exposes ${methodCount} methods instead of 18.`);
  await methodOptions.filter({ hasText: /^PLS-SEM Algorithm$/ }).click();
  const hocRoute = calculation.locator("#nd-calculation-higher-order");
  await hocRoute.waitFor({ state: "visible", timeout: 10_000 });
  const hocRouteText = compact(await hocRoute.textContent());
  assert(/Enterprise Capability/.test(hocRouteText) && /RR/.test(hocRouteText) && /disjoint two-stage/i.test(hocRouteText), `Calculate did not route the exact HOC: ${hocRouteText}`);
  const start = calculation.getByRole("button", { name: "Start calculation", exact: true });
  assert(await start.isEnabled(), `HOC calculation is blocked: ${compact(await calculation.textContent())}`);
  await capture(page, report, screenshotRoot, "03-calculate", "Calculate keeps the 18-method catalogue and exposes one compact RR disjoint-two-stage HOC route row under PLS-SEM Algorithm.");

  const saveHelper = createDialogHelper({
    python: args.python,
    mode: "save",
    target: args.projectPath,
    allowedRoot: RESULTS_ROOT,
    // A fresh calculation-ready project keeps the native HWND title at the
    // stable product name until its first archive publication. The document
    // title already includes the unsaved project name, so it is not a valid
    // owner-window binding for the Windows Save dialog at this point.
    windowTitle: "QuickPLS",
    extension: "qpls",
  });
  let saveCompleted = false;
  try {
    const ready = await saveHelper.ready;
    assert(ready?.passed && ready.event === "ready", `Native project Save helper was not ready: ${JSON.stringify(ready)}`);
    const progressCapture = (async () => {
      const monitor = page.locator(".nd-cbsem-v4-monitor");
      await monitor.waitFor({ state: "visible", timeout: 180_000 });
      const state = compact(await monitor.textContent());
      await capture(page, report, screenshotRoot, "04-progress", "The packaged HOC calculation is monitored through the native advanced-calculation progress surface.");
      return state;
    })();
    await start.click();
    const save = await saveHelper.completed;
    saveCompleted = true;
    assert(save?.passed, `Native project Save failed: ${JSON.stringify(save)}`);
    const progress = await progressCapture;
    const terminalState = await waitForCalculationTerminal(page);
    const results = await assertHocResults(page);
    await capture(page, report, screenshotRoot, "05-results", "The completed HOC calculation opens categorized Results with researcher-facing component relationships and retained canonical identity.");
    return {
      fixture,
      hocId,
      hocRoute: hocRouteText,
      progress,
      terminalState,
      save: { phase: save.phase, target: args.projectPath },
      results,
      processClose: "The packaged supervisor closes this execute process before the reopen phase.",
    };
  } finally {
    if (!saveCompleted) saveHelper.stop();
  }
}

async function reopenJourney(page, args, report, screenshotRoot) {
  assert(await pathExists(args.projectPath), `The saved HOC project is missing: ${args.projectPath}`);
  await page.evaluate(({ target }) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } }));
  }, { target: args.projectPath });
  await waitForSurface(page, "model", 60_000);
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
  const results = await assertHocResults(page);
  const expected = report.phases.execute?.checks?.results?.identity;
  assert(expected?.documentId === results.identity.documentId && expected?.runId === results.identity.runId,
    `Fresh reopen changed canonical HOC identity: ${JSON.stringify({ expected, actual: results.identity })}`);
  await capture(page, report, screenshotRoot, "06-reopen", "A newly launched packaged process reopens the saved archive and restores the same canonical HOC Results identity.");
  return { results, sameCanonicalIdentity: true };
}

const rawArgs = parseArgs(process.argv.slice(2));
const args = {
  ...rawArgs,
  evidenceDir: path.resolve(rawArgs["evidence-dir"]),
  projectPath: path.resolve(rawArgs["project-path"]),
  python: path.resolve(rawArgs.python),
};
assert(inside(RESULTS_ROOT, args.evidenceDir), "--evidence-dir must remain below validation/results.");
assert(inside(RESULTS_ROOT, args.projectPath), "--project-path must remain below validation/results.");
await fs.mkdir(args.evidenceDir, { recursive: true });
const screenshotRoot = path.join(args.evidenceDir, "screens");
await fs.mkdir(screenshotRoot, { recursive: true });
const reportPath = path.join(args.evidenceDir, "v252_hoc_packaged_smoke.json");
let report = rawArgs.phase === "reopen" && await pathExists(reportPath)
  ? JSON.parse(await fs.readFile(reportPath, "utf8"))
  : {
      schemaVersion: 1,
      version: "2.52.0",
      runtime: "Packaged Tauri WebView2 over local CDP",
      projectPath: path.relative(ROOT, args.projectPath).split(path.sep).join("/"),
      generatedAt: new Date().toISOString(),
      complete: false,
      passed: false,
      screenshots: [],
      observations: [],
      phases: {},
      failures: [],
    };
const phase = { passed: false, checks: null, offline: null, consoleErrors: [], failures: [] };
let browser;
let offline;
try {
  if (rawArgs.phase === "reopen") {
    assert(report.phases.execute?.passed === true, "Reopen requires a passing execute phase in the same report.");
  }
  const connection = await connectToSingleQuickPlsPage({ chromium, endpoint: rawArgs.endpoint });
  browser = connection.browser;
  const page = connection.page;
  offline = observeFunctionalOfflineRequests(page);
  page.on("pageerror", (error) => phase.consoleErrors.push({ type: "pageerror", message: error.message }));
  page.on("console", (message) => {
    if (message.type() === "error") phase.consoleErrors.push({ type: "console", message: message.text() });
  });
  page.on("dialog", (dialog) => dialog.accept());
  await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.setView === "function", null, { timeout: 30_000 });
  phase.checks = rawArgs.phase === "execute"
    ? await executeJourney(page, args, report, screenshotRoot)
    : await reopenJourney(page, args, report, screenshotRoot);
  phase.offline = offline.summary();
  assert(phase.offline.passed, `Packaged HOC smoke accessed an external origin: ${JSON.stringify(phase.offline)}`);
  assert(phase.consoleErrors.length === 0, `Packaged HOC console errors: ${JSON.stringify(phase.consoleErrors)}`);
  phase.passed = true;
} catch (error) {
  phase.failures.push(error instanceof Error ? error.message : String(error));
} finally {
  offline?.stop();
  await browser?.close().catch(() => undefined);
  report.phases[rawArgs.phase] = phase;
  report.generatedAt = new Date().toISOString();
  report.complete = report.phases.execute?.passed === true && report.phases.reopen?.passed === true;
  report.passed = report.complete;
  report.failures = Object.entries(report.phases).flatMap(([name, value]) =>
    (value.failures ?? []).map((message) => `${name}: ${message}`));
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
}

if (!phase.passed) {
  console.error(phase.failures[0] ?? `QuickPLS 2.52 HOC ${rawArgs.phase} phase failed.`);
  process.exit(1);
}
console.log(JSON.stringify({
  passed: phase.passed,
  complete: report.complete,
  phase: rawArgs.phase,
  reportPath,
  screenshots: report.screenshots,
}, null, 2));
