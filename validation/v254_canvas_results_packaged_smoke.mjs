#!/usr/bin/env node
/**
 * QuickPLS 2.54 packaged Canvas -> Calculate -> Results -> fresh-reopen journey.
 *
 * This driver intentionally exercises user-visible controls. It does not call
 * the model-edit gateway directly, and it records one structured observation
 * for each screenshot so the later product review has traceable evidence.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { execFileSync, spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import {
  PACKAGED_TAURI_ORIGIN,
  connectToSingleQuickPlsPage,
  observeFunctionalOfflineRequests,
} from "./v247_cdp_package_helpers.mjs";
import {
  canonicalIdentity,
  createEmptyModel,
  importFixture,
  waitForSurface,
} from "./general_sem_rank0_packaged_acceptance.mjs";
import { startPreview, waitForPreview } from "./lib/v2_ui_smoke_harness.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS_ROOT = path.join(ROOT, "validation", "results");
const FILE_DIALOG_HELPER = path.join(ROOT, "validation", "windows_native_owned_file_dialog.py");
const PROJECT_NAME = "QuickPLS 2.54 Canvas and Results packaged smoke";
const MODEL_NAME = "QuickPLS 2.54 moderated model";

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
  for (const key of ["phase", "evidence-dir"]) {
    if (!values[key]) throw new Error(`--${key} is required`);
  }
  if (!new Set(["headless", "execute", "reopen"]).has(values.phase)) {
    throw new Error("--phase must be headless, execute, or reopen");
  }
  if (values.phase !== "headless") {
    for (const key of ["endpoint", "project-path", "python"]) {
      if (!values[key]) throw new Error(`--${key} is required for ${values.phase}`);
    }
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

function escaped(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
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

async function createFixture(file, rowCount = 1_200) {
  const columns = ["x_1", "x_2", "y_1", "y_2", "w_1", "w_2", "z_1", "z_2"];
  const rows = Array.from({ length: rowCount }, (_, offset) => {
    const index = offset + 1;
    const x = Math.sin(index * 0.071) + Math.cos(index * 0.037) * 0.29 + index * 0.0002;
    const w = Math.cos(index * 0.059) - Math.sin(index * 0.023) * 0.31;
    const z = Math.sin(index * 0.043) + Math.cos(index * 0.031) * 0.27;
    const noise = Math.sin(index * 0.83) * 0.055 + Math.cos(index * 0.47) * 0.031;
    const y = 0.30 * x + 0.22 * w + 0.19 * z
      + 0.23 * x * w + 0.17 * x * z + 0.14 * w * z + 0.26 * x * w * z + noise;
    return [
      x,
      0.91 * x + 0.13 * noise,
      y,
      0.93 * y + 0.09 * noise,
      w,
      0.89 * w - 0.11 * noise,
      z,
      0.90 * z + 0.10 * noise,
    ].map((value) => value.toFixed(9)).join(",");
  });
  await fs.writeFile(file, `${columns.join(",")}\n${rows.join("\n")}\n`, { encoding: "utf8", flag: "wx" });
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

function constructNodes(page) {
  return page.locator(".react-flow__node-latent");
}

function constructNode(page, label) {
  return constructNodes(page).filter({ has: page.getByText(label, { exact: true }) });
}

function modelInspector(page) {
  return page.locator("aside.nd-model-inspector");
}

async function navigatorConstructRow(page, label) {
  const navigator = page.getByRole("complementary", { name: "Model navigator", exact: true });
  const constructsTab = navigator.getByRole("tab", { name: "Constructs", exact: true });
  await constructsTab.waitFor({ state: "visible", timeout: 10_000 });
  if (await constructsTab.getAttribute("aria-selected") !== "true") await constructsTab.click();
  const row = navigator.locator(".nd-model-object-list button").filter({
    hasText: new RegExp(`^${escaped(label)}\\s*\\d+\\s*$`),
  });
  await row.waitFor({ state: "visible", timeout: 10_000 });
  return row;
}

async function inspectorTab(page, label) {
  const tab = modelInspector(page).getByRole("tab", { name: label, exact: true });
  await tab.waitFor({ state: "visible", timeout: 10_000 });
  if (await tab.getAttribute("aria-selected") !== "true") await tab.click();
}

async function waitForConstructCount(page, expected) {
  await page.waitForFunction(
    (count) => document.querySelectorAll(".react-flow__node-latent").length === count,
    expected,
    { timeout: 15_000 },
  );
}

async function renameSelectedConstruct(page, label) {
  await inspectorTab(page, "Model");
  const inspector = modelInspector(page);
  const name = inspector.getByLabel("Name", { exact: true });
  const shortName = inspector.getByLabel("Short name", { exact: true });
  await name.fill(label);
  await name.press("Enter");
  await shortName.fill(label);
  await shortName.press("Enter");
  await constructNode(page, label).waitFor({ state: "visible", timeout: 10_000 });
}

async function selectIndicatorPair(page, indicators) {
  const list = page.getByRole("listbox", { name: "Dataset indicators", exact: true });
  await list.waitFor({ state: "visible", timeout: 10_000 });
  const options = indicators.map((indicator) => list.getByRole("option", {
    name: new RegExp(`^${escaped(indicator)}\\.`),
  }));
  await options[0].click();
  await options[1].click({ modifiers: ["Control"] });
  const selection = page.getByRole("group", { name: "2 selected indicators", exact: true });
  await selection.waitFor({ state: "visible", timeout: 10_000 });
  assert(await options[0].getAttribute("aria-selected") === "true", `${indicators[0]} was not selected.`);
  assert(await options[1].getAttribute("aria-selected") === "true", `${indicators[1]} was not selected.`);
  return selection;
}

async function createConstructFromIndicators(page, indicators, label, beforeCreate) {
  const countBefore = await constructNodes(page).count();
  const idsBefore = new Set(await constructNodes(page).evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-id"))));
  const selection = await selectIndicatorPair(page, indicators);
  if (beforeCreate) await beforeCreate(selection);
  await selection.getByRole("button", { name: "Create construct", exact: true }).click();
  await waitForConstructCount(page, countBefore + 1);
  const idsAfter = await constructNodes(page).evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-id")));
  const createdIds = idsAfter.filter((id) => id && !idsBefore.has(id));
  assert(createdIds.length === 1, `Expected one newly created construct ID; found ${JSON.stringify(createdIds)}.`);
  const created = page.locator(`.react-flow__node-latent[data-id="${createdIds[0]}"]`);
  await created.click();
  await renameSelectedConstruct(page, label);
  return constructNode(page, label);
}

async function selectConstructs(page, labels) {
  await (await navigatorConstructRow(page, labels[0])).click();
  for (const label of labels.slice(1)) {
    await constructNode(page, label).click({ modifiers: ["Control"], position: { x: 12, y: 40 } });
  }
  const selected = page.locator(".react-flow__node-latent.selected");
  await page.waitForFunction(
    (count) => document.querySelectorAll(".react-flow__node-latent.selected").length === count,
    labels.length,
    { timeout: 10_000 },
  );
  assert(await selected.count() === labels.length, `Expected ${labels.length} selected constructs.`);
}

async function alignConstructsTop(page, labels) {
  await selectConstructs(page, labels);
  await page.getByRole("button", { name: "Arrange options", exact: true }).click();
  const menu = page.getByRole("menu", { name: "Arrange options", exact: true });
  await menu.waitFor({ state: "visible", timeout: 10_000 });
  const command = menu.getByRole("menuitem", { name: "Align top", exact: true });
  assert(await command.isEnabled(), "Align top is disabled for a four-construct selection.");
  await command.click();
  await page.waitForTimeout(200);
  const topValues = [];
  for (const label of labels) {
    const bounds = await constructNode(page, label).boundingBox();
    assert(bounds, `Construct ${label} has no rendered bounds after alignment.`);
    topValues.push(bounds.y);
  }
  const spread = Math.max(...topValues) - Math.min(...topValues);
  assert(spread <= 2, `Align top left a ${spread.toFixed(2)}px vertical spread.`);
  return { selectedCount: labels.length, topSpreadPx: Number(spread.toFixed(2)) };
}

async function confirmCompositeRepresentations(page, labels) {
  const inspector = modelInspector(page);
  const expert = inspector.getByRole("button", { name: "Expert", exact: true });
  if (await expert.getAttribute("aria-pressed") !== "true") await expert.click();
  for (const label of labels) {
    await (await navigatorConstructRow(page, label)).click();
    await inspector.getByRole("tab", { name: "Parameter", exact: true }).click();
    const authoring = inspector.locator(".nd-sem-authoring");
    await authoring.waitFor({ state: "visible", timeout: 10_000 });
    const representation = authoring.locator('select[id$="-representation"]');
    await representation.selectOption("composite");
    assert(await representation.inputValue() === "composite", `${label} was not confirmed as a PLS composite.`);
  }
}

async function createPath(page, sourceLabel, targetLabel) {
  await page.getByRole("button", { name: "Connect", exact: true }).click();
  await constructNode(page, sourceLabel).dispatchEvent("click");
  await constructNode(page, targetLabel).dispatchEvent("click");
  await scientificPaths(page).first().waitFor({ state: "attached", timeout: 10_000 });
  assert(await scientificPaths(page).count() === 1, "Expected one persisted focal structural path.");
}

async function openModerationFromSelectedPath(page) {
  const focal = scientificPaths(page).first();
  const interaction = focal.locator(".react-flow__edge-interaction");
  const bounds = await interaction.boundingBox();
  assert(bounds, "The focal path has no generous interaction hit area.");
  await interaction.dispatchEvent("click", {
    clientX: bounds.x + bounds.width / 2,
    clientY: bounds.y + bounds.height / 2,
  });
  await page.keyboard.press("m");
  const dialog = page.getByRole("dialog", { name: "Create Moderating Effect", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  return dialog;
}

async function createTwoWayModeration(page) {
  const dialog = await openModerationFromSelectedPath(page);
  await dialog.locator("#nd-moderation-moderator").selectOption({ label: "W" });
  const summary = compact(await dialog.locator(".nd-moderation-summary").textContent());
  assert(/Two-way moderation/i.test(summary) && /X.*Y/i.test(summary), `Unexpected two-way summary: ${summary}`);
  const add = dialog.getByRole("button", { name: "Add moderating effect", exact: true });
  assert(await add.isEnabled(), `Two-way moderation is blocked: ${compact(await dialog.textContent())}`);
  return { dialog, summary, add };
}

async function createThreeWayModeration(page) {
  const anchor = page.locator('.moderation-anchor[role="button"]');
  await anchor.waitFor({ state: "visible", timeout: 10_000 });
  assert(await anchor.count() === 1, "Expected one two-way moderation anchor before extension.");
  await anchor.click();
  await page.keyboard.press("m");
  const dialog = page.getByRole("dialog", { name: "Create Moderating Effect", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.locator("#nd-moderation-moderator").selectOption({ label: "Z" });
  const summary = compact(await dialog.locator(".nd-moderation-summary").textContent());
  assert(/Three-way moderation/i.test(summary), `Unexpected three-way summary: ${summary}`);
  const add = dialog.getByRole("button", { name: "Add three-way effect", exact: true });
  assert(await add.isEnabled(), `Three-way moderation is blocked: ${compact(await dialog.textContent())}`);
  return { dialog, summary, add };
}

async function saveNativeProjectAction(page, python, target, trigger) {
  const helper = createDialogHelper({ python, target });
  let completed = false;
  try {
    const ready = await helper.ready;
    assert(ready?.passed && ready.event === "ready", `Native Save helper was not ready: ${JSON.stringify(ready)}`);
    await trigger();
    const save = await helper.completed;
    completed = true;
    assert(save?.passed, `Native project Save failed: ${JSON.stringify(save)}`);
    return save;
  } finally {
    if (!completed) helper.stop();
  }
}

async function prepareCalculationReadyRevision(page, args) {
  const sourceRevisionPath = path.join(args.evidenceDir, "quickpls-v254-canvas-results-two-way-source.qpls");
  assert(!await pathExists(sourceRevisionPath), `Source revision path must be new: ${sourceRevisionPath}`);
  await page.getByRole("menubar", { name: "Application menu", exact: true })
    .getByRole("menuitem", { name: "Model", exact: true })
    .click();
  const command = page.getByRole("menuitem", { name: "Create Calculation-Ready Revision…", exact: true });
  if (await command.isVisible().catch(() => false)) {
    await command.click();
  } else {
    const parameters = page.getByRole("menuitem", { name: "Advanced Parameter Table…", exact: true });
    await parameters.waitFor({ state: "visible", timeout: 10_000 });
    await parameters.click();
    const parameterDialog = page.getByRole("dialog", { name: "Advanced Parameter Table", exact: true });
    await parameterDialog.waitFor({ state: "visible", timeout: 30_000 });
    await parameterDialog.getByRole("button", { name: "Continue to Calculate", exact: true }).click();
  }
  const advanced = page.getByRole("dialog", { name: "Calculate Advanced Model", exact: true });
  await advanced.waitFor({ state: "visible", timeout: 30_000 });
  const activate = advanced.locator("button.primary:not([disabled])").filter({ hasText: /^Save and activate project…$/ });
  await activate.waitFor({ state: "visible", timeout: 30_000 });
  const save = await saveNativeProjectAction(page, args.python, sourceRevisionPath, () => activate.click());
  const close = advanced.getByRole("button", { name: "Close dialog", exact: true });
  await close.waitFor({ state: "visible", timeout: 30_000 });
  await close.click();
  await advanced.waitFor({ state: "hidden", timeout: 10_000 });
  return { sourceRevisionPath, save };
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
        throw new Error(`Three-way calculation ${value}: ${compact(await page.locator(".nd-cbsem-v4-monitor").textContent())}`);
      }
    }
    await page.waitForTimeout(250);
  }
  throw new Error(`Three-way calculation did not finish: ${compact(await page.locator(".nd-cbsem-v4-monitor").textContent())}`);
}

async function waitForUnifiedCalculationEntry(page, timeout = 15_000) {
  const advanced = page.getByRole("dialog", { name: "Calculate Advanced Model", exact: true });
  const monitor = page.locator(".nd-cbsem-v4-monitor");
  const results = page.locator(".nd-results-workspace");
  const deadline = Date.now() + timeout;
  let advancedText = "";
  while (Date.now() < deadline) {
    if (await results.isVisible().catch(() => false)) return "results";
    if (await monitor.isVisible().catch(() => false)) return "monitor";
    if (await advanced.isVisible().catch(() => false)) {
      advancedText = compact(await advanced.textContent());
    }
    await page.waitForTimeout(100);
  }
  const dialogs = await page.locator('[role="dialog"]').allTextContents();
  throw new Error(`Unified calculation did not open its monitor or Results: ${JSON.stringify({ advancedText, dialogs, body: compact(await page.locator("body").textContent()).slice(-1500) })}`);
}

async function assertThreeWayResults(page) {
  const workspace = page.locator(".nd-results-workspace");
  await workspace.waitFor({ state: "visible", timeout: 300_000 });
  const tree = page.getByRole("tree", { name: "Available result sections", exact: true });
  await tree.waitFor({ state: "visible", timeout: 30_000 });
  const group = tree.getByRole("treeitem", { name: /Three-Way Moderation/i }).first();
  await group.waitFor({ state: "visible", timeout: 30_000 });
  if (await group.getAttribute("aria-expanded") !== "true") await group.click();
  const item = tree.getByRole("treeitem", { name: /^Three-way interaction$/i }).first();
  await item.waitFor({ state: "visible", timeout: 15_000 });
  await item.click();
  const table = page.locator('[data-canonical-table-id="general_sem_three_way_effect"]');
  await table.waitFor({ state: "visible", timeout: 30_000 });
  const identity = await canonicalIdentity(page);
  assert(identity.documentId && identity.runId, `Canonical three-way identity is incomplete: ${JSON.stringify(identity)}`);
  const researcherText = compact(await workspace.textContent());
  assert(!/general_sem_v1_moderation_(?:term|output|main_relation|effect_relation|parameter)_/i.test(researcherText),
    "Normal Results exposed an internal generated moderation identity.");
  assert(!/general-sem:v1:interaction-(?:generated|dependency):/i.test(researcherText),
    "Normal Results exposed an internal moderation provenance annotation.");
  return {
    identity,
    tableId: await table.getAttribute("data-canonical-table-id"),
    groupLabel: compact(await group.textContent()),
    resultOverlayAnchors: await page.locator(".moderation-anchor.selected").count(),
  };
}

async function capture(page, report, screenshotRoot, observation) {
  const file = path.join(screenshotRoot, `${observation.id}.png`);
  await page.screenshot({ path: file, animations: "disabled" });
  const relative = path.relative(ROOT, file).split(path.sep).join("/");
  if (!report.screenshots.includes(relative)) report.screenshots.push(relative);
  report.observations = report.observations.filter((entry) => entry.id !== observation.id);
  report.observations.push({
    schema_version: 1,
    id: observation.id,
    phase: observation.phase,
    area: observation.area,
    expected: observation.expected,
    observed: observation.observed,
    status: "passed",
    severity: "none",
    recommendation: null,
    screenshot: relative,
  });
}

function stopTrackedProcessTree(process) {
  if (!process?.pid) return;
  try {
    // This PID is the cmd.exe returned by startPreview for this crawl. No
    // process-name or port-owner sweep is permitted here.
    execFileSync("taskkill.exe", ["/PID", String(process.pid), "/T", "/F"], { stdio: "ignore" });
  } catch {
    process.kill();
  }
}

async function previewAlreadyServing(url) {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(750) });
    return response.ok;
  } catch {
    return false;
  }
}

async function runHeadlessCrawl(rawArgs) {
  const evidenceDir = path.resolve(rawArgs["evidence-dir"]);
  assert(inside(RESULTS_ROOT, evidenceDir), "--evidence-dir must remain below validation/results.");
  await fs.mkdir(evidenceDir, { recursive: true });
  const reportPath = path.join(evidenceDir, "v254_canvas_results_headless_crawl.json");
  assert(!await pathExists(reportPath), `Headless crawl report must be new: ${reportPath}`);
  const port = Number(rawArgs.port ?? "57654");
  assert(Number.isInteger(port) && port >= 1024 && port <= 65535, `Invalid headless preview port: ${rawArgs.port}`);
  const baseUrl = `http://127.0.0.1:${port}/`;
  assert(!await previewAlreadyServing(baseUrl), `Headless preview port ${port} is already in use.`);

  const evidence = {
    schema_version: 1,
    suite_id: "quickpls_v254_canvas_results_headless_crawl_v1",
    version: "2.54.0",
    passed: false,
    started_at: new Date().toISOString(),
    completed_at: null,
    checks: [],
    console_errors: [],
    failures: [],
  };
  const record = (id, passed, details = {}) => {
    const check = { id, passed: Boolean(passed), ...details };
    evidence.checks.push(check);
    if (!check.passed) evidence.failures.push({ id, message: details.message ?? "check failed" });
  };
  const preview = startPreview(port);
  let browser;
  try {
    await waitForPreview(baseUrl, preview.logs);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
    page.on("pageerror", (error) => evidence.console_errors.push({ type: "pageerror", message: error.message }));
    page.on("console", (message) => {
      if (message.type() === "error") evidence.console_errors.push({ type: "console", message: message.text() });
    });
    await page.goto(`${baseUrl}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout: 30_000 });
    await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture === "function", null, { timeout: 15_000 });
    const fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadDiagramFixture("large"));
    await waitForSurface(page, "model", 30_000);
    const nodeCount = await constructNodes(page).count();
    record("canvas_fixture", nodeCount > 3, { nodeCount, fixture });

    const navigator = page.getByRole("complementary", { name: "Model navigator", exact: true });
    const navigatorTabs = navigator.getByRole("tab");
    record("model_navigator_tabs", await navigatorTabs.count() === 3, {
      labels: await navigatorTabs.allTextContents(),
    });
    await navigator.getByRole("tab", { name: "Constructs", exact: true }).click();
    const constructRows = navigator.locator(".nd-model-object-list button");
    await constructRows.first().waitFor({ state: "visible", timeout: 10_000 });
    await constructRows.first().click();
    const inspector = modelInspector(page);
    await inspector.waitFor({ state: "visible", timeout: 10_000 });
    record("canvas_to_inspector", /Construct/i.test(compact(await inspector.locator(".nd-pane-title").textContent())), {
      heading: compact(await inspector.locator(".nd-pane-title").textContent()),
    });
    await inspectorTab(page, "Appearance");
    record("appearance_controls", await inspector.getByText(/presentation-only|presentation only/i).count() > 0, {
      text: compact(await inspector.textContent()).slice(0, 800),
    });
    const arrangeToggle = page.getByRole("button", { name: "Arrange options", exact: true });
    await arrangeToggle.click();
    const arrangeMenu = page.getByRole("menu", { name: "Arrange options", exact: true });
    await arrangeMenu.waitFor({ state: "visible", timeout: 10_000 });
    const arrangeLabels = await arrangeMenu.getByRole("menuitem").allTextContents();
    record("arrange_menu", ["Tidy selection", "Align top", "Distribute horizontally"].every((label) => arrangeLabels.includes(label)), {
      labels: arrangeLabels,
    });
    await page.keyboard.press("Escape");

    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
    await waitForSurface(page, "results", 20_000);
    const results = page.locator(".nd-results-workspace");
    await results.waitFor({ state: "visible", timeout: 15_000 });
    const empty = results.locator('[data-results-empty-state="true"]');
    await empty.waitFor({ state: "visible", timeout: 15_000 });
    record("results_empty_state", await empty.isVisible(), { text: compact(await empty.textContent()) });
    const calculate = empty.getByRole("button", { name: "Calculate results", exact: true });
    await calculate.click();
    const calculation = page.getByRole("dialog", { name: "Calculate", exact: true });
    await calculation.waitFor({ state: "visible", timeout: 15_000 });
    const methodList = calculation.locator("#nd-calculation-method-list");
    await methodList.waitFor({ state: "visible", timeout: 15_000 });
    const methodCount = await methodList.locator('[role="option"]').count();
    record("results_to_calculate", methodCount === 18, { methodCount });
    await page.keyboard.press("Escape");
    await calculation.waitFor({ state: "hidden", timeout: 10_000 });

    record("console", evidence.console_errors.length === 0, { errors: evidence.console_errors });
  } catch (error) {
    evidence.failures.push({ id: "headless_exception", message: error instanceof Error ? error.message : String(error) });
  } finally {
    await browser?.close().catch(() => undefined);
    stopTrackedProcessTree(preview.server);
    evidence.completed_at = new Date().toISOString();
    evidence.passed = evidence.failures.length === 0 && evidence.console_errors.length === 0
      && evidence.checks.length >= 7 && evidence.checks.every((check) => check.passed);
    await fs.writeFile(reportPath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
  }
  assert(evidence.passed, `Headless Canvas/Results crawl failed: ${JSON.stringify(evidence.failures)}`);
  console.log(JSON.stringify({ passed: true, phase: "headless", reportPath, checks: evidence.checks.length }, null, 2));
}

async function executeJourney(page, args, report, screenshotRoot) {
  assert(!await pathExists(args.projectPath), `Execute project path must be new: ${args.projectPath}`);
  const fixturePath = path.join(args.evidenceDir, "canvas-results-input.csv");
  const fixture = await createFixture(fixturePath);
  await createUnifiedProject(page);
  await importFixture(page, args.python, fixturePath);
  await createEmptyModel(page, MODEL_NAME);

  await createConstructFromIndicators(page, ["x_1", "x_2"], "X", async () => {
    await capture(page, report, screenshotRoot, {
      id: "01-indicator-multiselect",
      phase: "execute",
      area: "Model navigator",
      expected: "Ctrl-selection retains two indicators and exposes one Create construct action.",
      observed: "x_1 and x_2 are simultaneously selected in the native indicator list.",
    });
  });
  await createConstructFromIndicators(page, ["y_1", "y_2"], "Y");
  await createConstructFromIndicators(page, ["w_1", "w_2"], "W");
  await createConstructFromIndicators(page, ["z_1", "z_2"], "Z");
  assert(await constructNodes(page).count() === 4, "The native indicator workflow did not create X, Y, W and Z.");

  await (await navigatorConstructRow(page, "X")).click();
  await inspectorTab(page, "Appearance");
  const indicatorPosition = modelInspector(page).locator("label", { hasText: "Indicator position" }).locator("select");
  await indicatorPosition.selectOption("left");
  assert(await indicatorPosition.inputValue() === "left", "The construct indicator side did not persist in the Inspector.");
  await capture(page, report, screenshotRoot, {
    id: "02-indicator-side",
    phase: "execute",
    area: "Properties / Appearance",
    expected: "A selected construct exposes one compact indicator-position selector.",
    observed: "X indicators are set to Left without changing the scientific measurement model.",
  });

  const alignment = await alignConstructsTop(page, ["X", "Y", "W", "Z"]);
  await capture(page, report, screenshotRoot, {
    id: "03-align",
    phase: "execute",
    area: "Canvas Arrange",
    expected: "Align top acts on the current multi-selection as one presentation edit.",
    observed: `Four constructs remain selected with a ${alignment.topSpreadPx}px top-edge spread.`,
  });

  await confirmCompositeRepresentations(page, ["X", "Y", "W", "Z"]);

  await createPath(page, "X", "Y");
  const twoWay = await createTwoWayModeration(page);
  await capture(page, report, screenshotRoot, {
    id: "04-two-way-dialog",
    phase: "execute",
    area: "Moderation authoring",
    expected: "M on the focal path opens the compact two-way moderation dialog.",
    observed: twoWay.summary,
  });
  await twoWay.add.click();
  await twoWay.dialog.waitFor({ state: "hidden", timeout: 10_000 });

  const calculationReadyRevision = await prepareCalculationReadyRevision(page, args);

  const threeWay = await createThreeWayModeration(page);
  await capture(page, report, screenshotRoot, {
    id: "05-three-way-dialog",
    phase: "execute",
    area: "Moderation authoring",
    expected: "M on the two-way anchor extends it with a distinct third moderator.",
    observed: threeWay.summary,
  });
  const threeWaySave = await saveNativeProjectAction(page, args.python, args.projectPath, () => threeWay.add.click());
  await threeWay.dialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.waitForFunction(() => {
    const anchor = document.querySelector('.moderation-anchor[role="button"]');
    return Boolean(anchor && /Z/i.test(anchor.getAttribute("aria-label") ?? ""));
  }, null, { timeout: 15_000 });
  const anchor = page.locator('.moderation-anchor[role="button"]');
  const anchorLabel = await anchor.getAttribute("aria-label");
  const connectorCount = await page.locator('.react-flow__edge[data-id^="moderation-connector::"]').count();
  assert(await anchor.count() === 1, "The maximal three-way effect should project as one compact anchor.");
  assert(/W/i.test(anchorLabel ?? "") && /Z/i.test(anchorLabel ?? ""), `Three-way anchor omits a moderator: ${anchorLabel}`);
  assert(connectorCount === 2, `Expected two visual-only moderator connectors; found ${connectorCount}.`);
  assert(await constructNodes(page).count() === 4, "Generated hierarchy constructs leaked onto the ordinary Canvas.");
  const scientificPathIds = await scientificPaths(page).evaluateAll((elements) => elements
    .map((element) => element.getAttribute("data-id") ?? ""));
  assert(scientificPathIds.length >= 1, "The authored focal relationship disappeared after three-way authoring.");
  assert(scientificPathIds.every((id) => !id.startsWith("moderation-connector::") && !id.startsWith("moderation-anchor::")),
    "A presentation anchor or connector contaminated persisted structural paths.");
  const visibleCanvasText = compact((await page.locator(".react-flow__node:visible").allTextContents()).join(" "));
  assert(!/general_sem_v1_moderation_(?:term|output|main_relation|effect_relation|parameter)_/i.test(visibleCanvasText),
    "Normal Canvas exposed an internal generated moderation identity.");
  assert(!/general-sem:v1:interaction-(?:generated|dependency):/i.test(visibleCanvasText),
    "Normal Canvas exposed an internal moderation provenance annotation.");
  assert(!/Two-stage moderation requires exactly one two-way interaction/i.test(await page.locator("body").textContent()),
    "A valid three-way model is still showing the obsolete two-way-only preflight blocker.");
  await capture(page, report, screenshotRoot, {
    id: "06-canvas",
    phase: "execute",
    area: "Model Canvas",
    expected: "One compact anchor and two dashed connectors represent the three-way effect without generated-node clutter.",
    observed: `${anchorLabel}; ${connectorCount} visual connectors; four visible constructs; ${scientificPathIds.length} authored/hierarchy scientific paths.`,
  });

  await page.keyboard.press("Control+R");
  const calculation = page.getByRole("dialog", { name: "Calculate", exact: true });
  await calculation.waitFor({ state: "visible", timeout: 15_000 });
  const methods = calculation.locator('#nd-calculation-method-list [role="option"]');
  await methods.first().waitFor({ state: "visible", timeout: 15_000 });
  assert(await methods.count() === 18, `Calculate exposes ${await methods.count()} methods instead of 18.`);
  await methods.filter({ hasText: /^PLS-SEM Algorithm$/ }).click();
  const route = calculation.locator('[id^="nd-calculation-three_way-moderation-"]').first();
  await route.waitFor({ state: "visible", timeout: 10_000 });
  const routeText = compact(await route.textContent());
  assert(/Three-way moderation/i.test(routeText), `Calculate did not expose the three-way route: ${routeText}`);
  const start = calculation.getByRole("button", { name: "Start calculation", exact: true });
  assert(await start.isEnabled(), `Three-way calculation is blocked: ${compact(await calculation.textContent())}`);
  await capture(page, report, screenshotRoot, {
    id: "07-calculate",
    phase: "execute",
    area: "Calculate",
    expected: "The unchanged 18-method catalogue routes PLS-SEM Algorithm to the detected three-way cell.",
    observed: `18 methods; ${routeText}`,
  });

  await start.click();
  const calculationEntry = await waitForUnifiedCalculationEntry(page);
  const progressCapture = (async () => {
    const monitor = page.locator(".nd-cbsem-v4-monitor");
    await monitor.waitFor({ state: "visible", timeout: 180_000 });
    const progress = compact(await monitor.textContent());
    await capture(page, report, screenshotRoot, {
      id: "08-progress",
      phase: "execute",
      area: "Calculation progress",
      expected: "Calculation uses the shared progress and cancellation surface.",
      observed: progress || "The shared calculation monitor is visible.",
    });
    return progress;
  })();
  const progress = await progressCapture;
  const terminalState = await waitForCalculationTerminal(page);
  const results = await assertThreeWayResults(page);
  await capture(page, report, screenshotRoot, {
    id: "09-results",
    phase: "execute",
    area: "Results",
    expected: "Successful calculation opens categorized, researcher-facing three-way Results.",
    observed: `${results.groupLabel}; canonical table ${results.tableId}.`,
  });
  return {
    fixture,
    alignment,
    anchorLabel,
    connectorCount,
    route: routeText,
    calculationEntry,
    progress,
    terminalState,
    calculationReadyRevision: {
      source: calculationReadyRevision.sourceRevisionPath,
      phase: calculationReadyRevision.save.phase,
    },
    save: { phase: threeWaySave.phase, target: args.projectPath },
    results,
  };
}

async function reopenJourney(page, args, report, screenshotRoot) {
  assert(await pathExists(args.projectPath), `Saved project is missing: ${args.projectPath}`);
  await page.evaluate(({ target }) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } }));
  }, { target: args.projectPath });
  await waitForSurface(page, "model", 60_000);
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
  const results = await assertThreeWayResults(page);
  const expected = report.phases.execute?.checks?.results?.identity;
  assert(expected?.documentId === results.identity.documentId && expected?.runId === results.identity.runId,
    `Fresh reopen changed canonical result identity: ${JSON.stringify({ expected, actual: results.identity })}`);
  await capture(page, report, screenshotRoot, {
    id: "10-reopen",
    phase: "reopen",
    area: "Fresh process reopen",
    expected: "A fresh packaged process restores the saved model and the same canonical result identity.",
    observed: `Document ${results.identity.documentId}; run ${results.identity.runId}; table ${results.tableId}.`,
  });
  return { results, sameCanonicalIdentity: true };
}

const rawArgs = parseArgs(process.argv.slice(2));
if (rawArgs.phase === "headless") {
  await runHeadlessCrawl(rawArgs);
  // The packaged execute/reopen authority below is intentionally not entered
  // during the preview crawl.
  process.exit(0);
}
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
const reportPath = path.join(args.evidenceDir, "v254_canvas_results_packaged_smoke.json");
let report = rawArgs.phase === "reopen" && await pathExists(reportPath)
  ? JSON.parse(await fs.readFile(reportPath, "utf8"))
  : {
      schema_version: 1,
      observation_schema: {
        schema_version: 1,
        required: ["id", "phase", "area", "expected", "observed", "status", "severity", "screenshot"],
        status_values: ["passed", "failed"],
        severity_values: ["none", "low", "medium", "high", "critical"],
      },
      suite_id: "quickpls_v254_canvas_results_packaged_smoke_v1",
      version: "2.54.0",
      runtime: "Packaged Tauri WebView2 over isolated local CDP",
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
  assert(phase.offline.passed, `Packaged smoke accessed an external origin: ${JSON.stringify(phase.offline)}`);
  assert(phase.consoleErrors.length === 0, `Packaged smoke console errors: ${JSON.stringify(phase.consoleErrors)}`);
  phase.passed = true;
} catch (error) {
  phase.failures.push(error instanceof Error ? error.message : String(error));
  if (page) {
    const failureScreenshot = path.join(screenshotRoot, `${rawArgs.phase}-failure.png`);
    await page.screenshot({ path: failureScreenshot, animations: "disabled" }).catch(() => undefined);
    phase.failureState = await page.evaluate(() => ({
      surface: document.querySelector(".nd-app")?.getAttribute("data-surface") ?? null,
      bodyText: (document.body?.innerText ?? "").replace(/\s+/g, " ").trim().slice(0, 4_000),
      selectedIndicatorCount: document.querySelectorAll('[role="option"][aria-selected="true"]').length,
      selectedConstructCount: document.querySelectorAll(".react-flow__node-latent.selected").length,
      anchorCount: document.querySelectorAll('.moderation-anchor[role="button"]').length,
      visibleConstructCount: document.querySelectorAll(".react-flow__node-latent").length,
      canvasEdgeIds: Array.from(document.querySelectorAll('.react-flow__edge[data-id]')).map((edge) => edge.getAttribute("data-id")),
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
  console.error(phase.failures[0] ?? `QuickPLS 2.54 Canvas/Results ${rawArgs.phase} phase failed.`);
  process.exit(1);
}
console.log(JSON.stringify({
  passed: phase.passed,
  complete: report.complete,
  phase: rawArgs.phase,
  reportPath,
  screenshots: report.screenshots,
  observations: report.observations.length,
}, null, 2));
