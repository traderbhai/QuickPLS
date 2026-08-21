#!/usr/bin/env node
/** QuickPLS 2.53 packaged moderation create -> calculate -> Results -> reopen. */

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
const PROJECT_NAME = "QuickPLS 2.53 moderation packaged smoke";
const MODEL_NAME = "QuickPLS 2.53 moderation model";

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
  if (!new Set(["execute", "reopen"]).has(values.phase)) throw new Error("--phase must be execute or reopen");
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

function createDialogHelper({ python, target }) {
  const child = spawn(python, [
    FILE_DIALOG_HELPER,
    "--mode", "save",
    "--target", target,
    "--allowed-root", RESULTS_ROOT,
    "--window-title", "QuickPLS",
    "--timeout-seconds", "90",
    "--extension", "qpls",
  ], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  let pending = "";
  let stderr = "";
  let readyResolve;
  let completedResolve;
  let readySettled = false;
  let completedSettled = false;
  const ready = new Promise((resolve) => { readyResolve = resolve; });
  const completed = new Promise((resolve) => { completedResolve = resolve; });
  const settleReady = (event) => {
    if (!readySettled) { readySettled = true; readyResolve(event); }
  };
  const settleCompleted = (event) => {
    if (!completedSettled) { completedSettled = true; completedResolve(event); }
  };
  const accept = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      if (event.event === "ready") settleReady(event);
      if (event.event === "complete") {
        if (!event.passed) settleReady(event);
        settleCompleted(event);
      }
    } catch (error) {
      const failure = { event: "complete", passed: false, phase: "jsonl", message: String(error), line };
      settleReady(failure);
      settleCompleted(failure);
    }
  };
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    pending += chunk;
    const lines = pending.split(/\r?\n/);
    pending = lines.pop() ?? "";
    lines.forEach(accept);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.on("error", (error) => {
    const failure = { event: "complete", passed: false, phase: "spawn", message: error.message };
    settleReady(failure);
    settleCompleted(failure);
  });
  child.on("close", (code, signal) => {
    if (pending.trim()) accept(pending);
    const failure = { event: "complete", passed: false, phase: "exit", code, signal, stderr };
    settleReady(failure);
    settleCompleted(failure);
  });
  return { ready, completed, stop: () => child.kill() };
}

async function createFixture(file, rowCount = 600) {
  const columns = ["x_1", "x_2", "w_1", "w_2", "y_1", "y_2"];
  const rows = Array.from({ length: rowCount }, (_, offset) => {
    const index = offset + 1;
    const x = Math.sin(index * 0.071) + Math.cos(index * 0.037) * 0.29 + index * 0.0007;
    const w = Math.cos(index * 0.059) - Math.sin(index * 0.023) * 0.31;
    const noise = Math.sin(index * 0.83) * 0.055 + Math.cos(index * 0.47) * 0.031;
    const y = 0.39 * x + 0.24 * w + 0.31 * x * w + noise;
    return [x, 0.91 * x + 0.13 * noise, w, 0.89 * w - 0.11 * noise, y, 0.93 * y + 0.09 * noise]
      .map((value) => value.toFixed(9)).join(",");
  });
  await fs.writeFile(file, `${columns.join(",")}\n${rows.join("\n")}\n`, "utf8");
  return { columns, rowCount };
}

async function createUnifiedProject(page) {
  await page.keyboard.press("Control+n");
  const dialog = page.getByRole("dialog", { name: "New Project", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByLabel("Project name", { exact: true }).fill(PROJECT_NAME);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await waitForSurface(page, "data");
}

function scientificPaths(page) {
  return page.locator(
    '.react-flow__edge[data-id]:not([data-id^="measurement::"]):not([data-id^="hoc-membership::"]):not([data-id^="moderation-connector::"])',
  );
}

async function createPath(page, nodes, sourceIndex, targetIndex) {
  await page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ }).click();
  await nodes.nth(sourceIndex).dispatchEvent("click");
  await nodes.nth(targetIndex).dispatchEvent("click");
  await scientificPaths(page).first().waitFor({ state: "attached", timeout: 10_000 });
  assert(await scientificPaths(page).count() === 1, "Expected one persisted focal structural path.");
}

async function capture(page, report, screenshotRoot, id, observation) {
  const file = path.join(screenshotRoot, `${id}.png`);
  await page.screenshot({ path: file, animations: "disabled" });
  const relative = path.relative(ROOT, file).split(path.sep).join("/");
  if (!report.screenshots.includes(relative)) report.screenshots.push(relative);
  report.observations = report.observations.filter((entry) => entry.id !== id);
  report.observations.push({ id, observation });
}

async function createModeration(page, report, screenshotRoot) {
  const focal = scientificPaths(page).first();
  const interaction = focal.locator(".react-flow__edge-interaction");
  const bounds = await interaction.boundingBox();
  assert(bounds, "The focal path has no generous interaction hit area.");
  await interaction.dispatchEvent("contextmenu", {
    button: 2,
    clientX: bounds.x + bounds.width / 2,
    clientY: bounds.y + bounds.height / 2,
  });
  const menu = page.getByRole("menu", { name: "Workspace commands", exact: true });
  await menu.waitFor({ state: "visible", timeout: 10_000 });
  const command = menu.getByRole("menuitem", { name: /^Moderating Effect/ });
  assert(await command.isEnabled(), `Moderating Effect command is disabled: ${await command.getAttribute("aria-label")}`);
  await command.click();
  const dialog = page.getByRole("dialog", { name: "Create Moderating Effect", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.locator("#nd-moderation-moderator").selectOption({ label: "W" });
  const summary = compact(await dialog.locator(".nd-moderation-summary").textContent());
  assert(/Two-way moderation/i.test(summary) && /X.*Y/i.test(summary), `Unexpected moderation summary: ${summary}`);
  await capture(page, report, screenshotRoot, "01-moderation-dialog", "The compact path-scoped dialog binds W to X -> Y and keeps construction details collapsed.");
  const add = dialog.getByRole("button", { name: "Add moderating effect", exact: true });
  assert(await add.isEnabled(), `Moderation submission is blocked: ${compact(await dialog.textContent())}`);
  await add.click();
  await dialog.waitFor({ state: "hidden", timeout: 10_000 });
  const anchor = page.locator('.moderation-anchor[role="button"]');
  await anchor.waitFor({ state: "visible", timeout: 10_000 });
  assert(await anchor.count() === 1, "Expected exactly one moderation anchor.");
  const anchorLabel = await anchor.getAttribute("aria-label");
  assert(/W.*moderates.*X.*Y/i.test(anchorLabel ?? ""), `Anchor has no researcher-facing accessible identity: ${anchorLabel}`);
  assert(await page.locator(".react-flow__node-latent").count() === 3, "A generated interaction construct leaked onto the ordinary Canvas.");
  assert(await page.locator('.react-flow__edge[data-id^="moderation-connector::"]').count() === 1, "Expected one dashed visual-only moderator connector.");
  return anchorLabel;
}

async function waitForCalculationTerminal(page, timeout = 180_000) {
  const state = page.locator(".nd-cbsem-v4-state");
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await page.locator(".nd-results-workspace").isVisible().catch(() => false)) return "completed";
    if (await state.isVisible().catch(() => false)) {
      const value = compact(await state.textContent().catch(() => "")).toLowerCase();
      if (value === "completed") return value;
      if (value === "failed" || value === "cancelled") {
        throw new Error(`Moderation calculation ${value}: ${compact(await page.locator(".nd-cbsem-v4-monitor").textContent())}`);
      }
    }
    await page.waitForTimeout(250);
  }
  throw new Error(`Moderation calculation did not finish: ${compact(await page.locator(".nd-cbsem-v4-monitor").textContent())}`);
}

async function assertModerationResults(page) {
  const workspace = page.locator(".nd-results-workspace");
  await workspace.waitFor({ state: "visible", timeout: 300_000 });
  const tree = page.getByRole("tree", { name: "Available result sections" });
  await tree.waitFor({ state: "visible", timeout: 30_000 });
  const treeText = compact(await tree.textContent());
  assert(
    /Moderation and Conditional Effects/i.test(treeText),
    `Results lacks the Moderation and Conditional Effects category: ${treeText}`,
  );
  const resultItem = tree.getByRole("treeitem", { name: /Interaction effects and product scaling/i }).first();
  if (await resultItem.count()) await resultItem.click();
  const table = page.locator('[data-canonical-table-id="general_sem_interaction_effects"]');
  await table.waitFor({ state: "visible", timeout: 30_000 });
  const identity = await canonicalIdentity(page);
  assert(identity.documentId && identity.runId, `Canonical moderation identity is incomplete: ${JSON.stringify(identity)}`);
  return {
    identity,
    tableId: await table.getAttribute("data-canonical-table-id"),
    treeText,
    overlayAnchorCount: await page.locator(".moderation-anchor.selected, .moderation-result-highlight .moderation-anchor").count(),
  };
}

async function executeJourney(page, args, report, screenshotRoot) {
  assert(!await pathExists(args.projectPath), `Execute project path must be new: ${args.projectPath}`);
  const fixturePath = path.join(args.evidenceDir, "moderation-input.csv");
  const fixture = await createFixture(fixturePath);
  await createUnifiedProject(page);
  await importFixture(page, args.python, fixturePath);
  await createEmptyModel(page, MODEL_NAME);
  const nodes = await buildConstructs(page, [
    { name: "X", indicators: ["x_1", "x_2"] },
    { name: "W", indicators: ["w_1", "w_2"] },
    { name: "Y", indicators: ["y_1", "y_2"] },
  ]);
  assert(await nodes.count() === 3, "The packaged smoke did not create X, W and Y.");
  await createPath(page, nodes, 0, 2);
  const anchorLabel = await createModeration(page, report, screenshotRoot);
  await capture(page, report, screenshotRoot, "02-canvas", "Canvas shows a compact x anchor and dashed W connector while the generated interaction construct stays hidden.");

  await page.keyboard.press("Control+R");
  const calculation = page.getByRole("dialog", { name: "Calculate", exact: true });
  await calculation.waitFor({ state: "visible", timeout: 15_000 });
  const methods = calculation.locator('#nd-calculation-method-list [role="option"]');
  await methods.first().waitFor({ state: "visible", timeout: 15_000 });
  assert(await methods.count() === 18, `Calculate exposes ${await methods.count()} methods instead of 18.`);
  await methods.filter({ hasText: /^PLS-SEM Algorithm$/ }).click();
  const route = calculation.locator("#nd-calculation-two_way-moderation-0");
  await route.waitFor({ state: "visible", timeout: 10_000 });
  const routeText = compact(await route.textContent());
  assert(/Moderation/i.test(routeText) && /W.*X.*Y/i.test(routeText), `Calculate did not show the exact moderation route: ${routeText}`);
  const start = calculation.getByRole("button", { name: "Start calculation", exact: true });
  assert(await start.isEnabled(), `Moderation calculation is blocked: ${compact(await calculation.textContent())}`);
  await capture(page, report, screenshotRoot, "03-calculate", "Calculate retains exactly 18 methods and shows one concise W-moderates-X-to-Y route row.");

  const saveHelper = createDialogHelper({ python: args.python, target: args.projectPath });
  let saveCompleted = false;
  try {
    const ready = await saveHelper.ready;
    assert(ready?.passed && ready.event === "ready", `Native Save helper was not ready: ${JSON.stringify(ready)}`);
    const progressCapture = (async () => {
      const monitor = page.locator(".nd-cbsem-v4-monitor");
      await monitor.waitFor({ state: "visible", timeout: 180_000 });
      const state = compact(await monitor.textContent());
      await capture(page, report, screenshotRoot, "04-progress", "The packaged moderation calculation uses the shared native progress surface.");
      return state;
    })();
    await start.click();
    const save = await saveHelper.completed;
    saveCompleted = true;
    assert(save?.passed, `Native project Save failed: ${JSON.stringify(save)}`);
    const progress = await progressCapture;
    const terminalState = await waitForCalculationTerminal(page);
    const results = await assertModerationResults(page);
    await capture(page, report, screenshotRoot, "05-results", "The completed calculation opens categorized Moderation Results backed by the canonical interaction table.");
    return {
      fixture,
      anchorLabel,
      route: routeText,
      progress,
      terminalState,
      save: { phase: save.phase, target: args.projectPath },
      results,
    };
  } finally {
    if (!saveCompleted) saveHelper.stop();
  }
}

async function reopenJourney(page, args, report, screenshotRoot) {
  assert(await pathExists(args.projectPath), `Saved moderation project is missing: ${args.projectPath}`);
  await page.evaluate(({ target }) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } }));
  }, { target: args.projectPath });
  await waitForSurface(page, "model", 60_000);
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
  const results = await assertModerationResults(page);
  const expected = report.phases.execute?.checks?.results?.identity;
  assert(expected?.documentId === results.identity.documentId && expected?.runId === results.identity.runId,
    `Fresh reopen changed canonical moderation identity: ${JSON.stringify({ expected, actual: results.identity })}`);
  await capture(page, report, screenshotRoot, "06-reopen", "A new packaged process reopens the saved project with the same moderation result identity.");
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
const reportPath = path.join(args.evidenceDir, "v253_mediation_moderation_packaged_smoke.json");
let report = rawArgs.phase === "reopen" && await pathExists(reportPath)
  ? JSON.parse(await fs.readFile(reportPath, "utf8"))
  : {
      schema_version: 1,
      version: "2.53.0",
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
let page;
let offline;
try {
  if (rawArgs.phase === "reopen") assert(report.phases.execute?.passed === true, "Reopen requires a passing execute phase.");
  const connection = await connectToSingleQuickPlsPage({ chromium, endpoint: rawArgs.endpoint });
  browser = connection.browser;
  page = connection.page;
  offline = observeFunctionalOfflineRequests(page);
  page.on("pageerror", (error) => phase.consoleErrors.push({ type: "pageerror", message: error.message, stack: error.stack ?? null }));
  page.on("console", (message) => {
    if (message.type() === "error") phase.consoleErrors.push({ type: "console", message: message.text() });
  });
  page.on("dialog", (dialog) => dialog.accept());
  await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.setView === "function", null, { timeout: 30_000 });
  phase.checks = rawArgs.phase === "execute"
    ? await executeJourney(page, args, report, screenshotRoot)
    : await reopenJourney(page, args, report, screenshotRoot);
  phase.offline = offline.summary();
  assert(phase.offline.passed, `Packaged moderation smoke accessed an external origin: ${JSON.stringify(phase.offline)}`);
  assert(phase.consoleErrors.length === 0, `Packaged moderation console errors: ${JSON.stringify(phase.consoleErrors)}`);
  phase.passed = true;
} catch (error) {
  phase.failures.push(error instanceof Error ? error.message : String(error));
  if (page) {
    const failureScreenshot = path.join(screenshotRoot, `${rawArgs.phase}-failure.png`);
    await page.screenshot({ path: failureScreenshot, animations: "disabled" }).catch(() => undefined);
    phase.failureState = await page.evaluate(() => ({
      bodyText: (document.body?.innerText ?? "").replace(/\s+/g, " ").trim().slice(0, 4_000),
      anchorCount: document.querySelectorAll('.moderation-anchor[role="button"]').length,
      interactionNodeCount: document.querySelectorAll('.react-flow__node[data-id^="interaction-"]').length,
      canvasNodeCount: document.querySelectorAll('.react-flow__node').length,
      canvasEdgeIds: Array.from(document.querySelectorAll('.react-flow__edge[data-id]')).map((edge) => edge.getAttribute('data-id')),
    })).catch(() => null);
  }
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
  console.error(phase.failures[0] ?? `QuickPLS 2.53 moderation ${rawArgs.phase} phase failed.`);
  process.exit(1);
}
console.log(JSON.stringify({
  passed: phase.passed,
  complete: report.complete,
  phase: rawArgs.phase,
  reportPath,
  screenshots: report.screenshots,
}, null, 2));
