#!/usr/bin/env node
/** Genuine packaged-Tauri General SEM Rank 0 scenario driver.
 *
 * The PowerShell package supervisor launches one installed/portable process and
 * invokes this driver for either the primary create/execute/export/append/close
 * flow or a fresh-process reopen/accessibility/viewport flow.  This file never
 * fabricates final acceptance booleans; it writes raw observations consumed by
 * general_sem_rank0_packaged_runner.py.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import {
  PACKAGED_TAURI_ORIGIN,
  connectToSingleQuickPlsPage,
  observeFunctionalOfflineRequests,
  setActualTauriClientViewport,
} from "./v247_cdp_package_helpers.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS_ROOT = path.join(ROOT, "validation", "results");
const FILE_DIALOG_HELPER = path.join(ROOT, "validation", "windows_native_owned_file_dialog.py");
const PACKAGED_MAIN_WINDOW_TITLE = "QuickPLS";
const ARCHIVE_IDENTITY_HELPER = path.join(ROOT, "validation", "general_sem_rank0_schema6_archive_identity.py");
const VIEWPORTS = [
  { id: "1024x700", width: 1024, height: 700 },
  { id: "1280x720", width: 1280, height: 720 },
  { id: "1440x900", width: 1440, height: 900 },
];
const VARIANTS = Object.freeze({
  mediation_point: { family: "mediation", bootstrap: false },
  multiple_mediation_bootstrap: { family: "mediation", bootstrap: true },
  multiple_two_way_moderation_point: { family: "moderation", bootstrap: false },
  multiple_two_way_moderation_bootstrap: { family: "moderation", bootstrap: true },
});

function parseArgs(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) throw new Error(`Unexpected positional argument: ${token}`);
    const key = token.slice(2);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for --${key}`);
    values[key] = value;
    index += 1;
  }
  const required = ["phase", "package-kind", "variant-id", "endpoint", "evidence-dir", "project-path", "python"];
  for (const key of required) if (!values[key]) throw new Error(`--${key} is required`);
  if (!['execute', 'reopen'].includes(values.phase)) throw new Error("--phase must be execute or reopen");
  if (!['installed', 'portable'].includes(values['package-kind'])) throw new Error("--package-kind must be installed or portable");
  if (!VARIANTS[values['variant-id']]) throw new Error(`Unknown Rank 0 variant ${values['variant-id']}`);
  if (values.phase === "reopen" && !values["scale-percent"]) throw new Error("--scale-percent is required for reopen");
  if (values.phase === "reopen" && !values["identity-file"]) throw new Error("--identity-file is required for reopen");
  return values;
}

function sha256(bytes) { return createHash("sha256").update(bytes).digest("hex"); }
function compact(value) { return String(value ?? "").replace(/\s+/g, " ").trim(); }

async function schema6ArchiveIdentity(python, archivePath) {
  const child = spawn(python, [ARCHIVE_IDENTITY_HELPER, "--archive", archivePath], {
    cwd: ROOT,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  const exit = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", (code, signal) => resolve({ code, signal }));
  });
  let value;
  try { value = JSON.parse(stdout); }
  catch (error) { throw new Error(`Schema-6 archive identity output is invalid: ${error}; stderr=${stderr}`); }
  if (exit.code !== 0
    || value?.schema_version !== 1
    || value?.evidence_kind !== "general_sem_rank0_schema6_archive_identity"
    || path.resolve(value?.archive_path ?? "") !== path.resolve(archivePath)
    || !Number.isSafeInteger(value?.byte_length)
    || value.byte_length <= 0
    || !/^[0-9a-f]{64}$/.test(value?.sha256 ?? "")
    || value?.project_schema_version !== 6
    || value?.sem_generation !== "general_sem_v1"
    || !Number.isSafeInteger(value?.canonical_result_attachment_count)
    || value.canonical_result_attachment_count < 0) {
    throw new Error(`Schema-6 archive identity failed closed: ${JSON.stringify({ exit, value, stderr })}`);
  }
  return value;
}

async function writeJsonNew(file, value) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  const handle = await fs.open(file, "wx");
  try { await handle.writeFile(`${JSON.stringify(value, null, 2)}\n`, "utf8"); }
  finally { await handle.close(); }
}

async function fileIdentity(file) {
  const bytes = await fs.readFile(file);
  return { path: file, size: bytes.length, sha256: sha256(bytes) };
}

function helperProcess({ python, mode, target, extensions, windowTitle, timeoutSeconds = 45, ownerPid = null, ownerExecutable = null }) {
  const args = [
    FILE_DIALOG_HELPER,
    "--mode", mode,
    "--target", target,
    "--allowed-root", RESULTS_ROOT,
    "--window-title", windowTitle,
    "--timeout-seconds", String(timeoutSeconds),
  ];
  if ((ownerPid === null) !== (ownerExecutable === null)) {
    throw new Error("Native file-dialog owner PID and executable must be supplied together.");
  }
  if (ownerPid !== null) {
    if (!Number.isSafeInteger(ownerPid) || ownerPid <= 0 || !path.isAbsolute(ownerExecutable)) {
      throw new Error("Native file-dialog owner identity is invalid.");
    }
    args.push("--owner-pid", String(ownerPid), "--owner-executable", ownerExecutable);
  }
  for (const extension of extensions) args.push("--extension", extension);
  const child = spawn(python, args, { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  const events = [];
  let stdout = "";
  let stderr = "";
  let readyResolve;
  let completeResolve;
  let readySettled = false;
  let completeSettled = false;
  const ready = new Promise((resolve) => { readyResolve = resolve; });
  const completed = new Promise((resolve) => { completeResolve = resolve; });
  const settleReady = (event) => { if (!readySettled) { readySettled = true; readyResolve(event); } };
  const settleComplete = (event) => { if (!completeSettled) { completeSettled = true; completeResolve(event); } };
  const acceptLine = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      events.push(event);
      if (event.event === "ready") settleReady(event);
      if (event.event === "complete") { if (!event.passed) settleReady(event); settleComplete(event); }
    } catch (error) {
      const failure = { event: "complete", passed: false, phase: "jsonl", error: { message: String(error) }, line };
      settleReady(failure); settleComplete(failure);
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
    const failure = { event: "complete", passed: false, phase: "spawn", error: { message: error.message } };
    settleReady(failure); settleComplete(failure);
  });
  child.on("close", (code, signal) => {
    if (stdout.trim()) acceptLine(stdout);
    if (!completeSettled) settleComplete({ event: "complete", passed: false, phase: "exit", code, signal, stderr, events });
    if (!readySettled) settleReady({ event: "complete", passed: false, phase: "exit_before_ready", code, signal, stderr, events });
  });
  return { ready, completed, stop: () => child.kill() };
}

async function withNativeDialog(page, args, action) {
  const helper = helperProcess(args);
  let completed = false;
  try {
    const ready = await helper.ready;
    if (!ready?.passed || ready.event !== "ready") throw new Error(`Native file helper was not ready: ${JSON.stringify(ready)}`);
    await action();
    const result = await helper.completed;
    completed = true;
    if (!result?.passed) throw new Error(`Native file dialog failed: ${JSON.stringify(result)}`);
    return result;
  } finally {
    if (!completed) helper.stop();
  }
}

async function fixtureFor(variantId, fixturePath) {
  const variant = VARIANTS[variantId];
  const family = variant.family;
  const columns = family === "mediation"
    ? ["x1", "x2", "m11", "m12", "m21", "m22", "y1", "y2"]
    : ["x1", "x2", "w1", "w2", "z1", "z2", "y1", "y2"];
  const rows = [];
  // Point estimators need a genuinely long packaged workload so the Cancel
  // control can be observed before completion. Bootstrap variants already gain
  // duration from full-refit inference and use a still-nontrivial applied row set.
  const rowCount = variant.bootstrap ? 1_000 : 100_000;
  for (let index = 1; index <= rowCount; index += 1) {
    const x = Math.sin(index * 0.19) + Math.cos(index * 0.07) * 0.3;
    const a = Math.cos(index * 0.23) + Math.sin(index * 0.11) * 0.2;
    const b = Math.sin(index * 0.31) - Math.cos(index * 0.13) * 0.25;
    const disturbance = Math.sin(index * 1.17) * 0.15 + Math.cos(index * 0.83) * 0.08;
    const y = family === "mediation"
      ? 0.25 * x + 0.46 * (0.62 * x + a * 0.35) + 0.38 * (0.51 * x + b * 0.4) + disturbance
      : 0.45 * x + 0.27 * a + 0.2 * b + 0.24 * x * a - 0.18 * x * b + disturbance;
    const values = family === "mediation"
      ? [x, x * 0.91 + disturbance * 0.2, 0.62 * x + a * 0.35, 0.58 * x + a * 0.4, 0.51 * x + b * 0.4, 0.49 * x + b * 0.43, y, y * 0.92 + disturbance * 0.25]
      : [x, x * 0.91 + disturbance * 0.2, a, a * 0.9 + disturbance * 0.15, b, b * 0.88 - disturbance * 0.1, y, y * 0.93 + disturbance * 0.2];
    rows.push(values.map((value) => value.toFixed(8)).join(","));
  }
  await fs.writeFile(fixturePath, `${columns.join(",")}\n${rows.join("\n")}\n`, { encoding: "utf8", flag: "wx" });
  return { ...(await fileIdentity(fixturePath)), rowCount, cancellationWorkload: true };
}

async function waitForSurface(page, surface, timeout = 20_000) {
  await page.locator(`.nd-app[data-surface="${surface}"]`).waitFor({ state: "visible", timeout });
}

async function createGeneralSemDraft(page, name, requireStandardAccess = false) {
  await page.keyboard.press("Control+n");
  const dialog = page.getByRole("dialog", { name: "New Project", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  const option = dialog.locator('input[name="project-type"][value="general_sem_v1"]');
  await option.waitFor({ state: "visible", timeout: 10_000 });
  if (!await option.isEnabled()) throw new Error("General SEM project mode is unavailable for the requested Registry access.");
  const labsChipCount = await dialog.locator(".nd-experimental-chip").count();
  if (requireStandardAccess && labsChipCount) throw new Error("Standard General SEM is still presented as Labs.");
  if (!requireStandardAccess && labsChipCount === 0) throw new Error("Pre-promotion General SEM did not retain its Labs label.");
  await option.check();
  await dialog.getByLabel("Project name", { exact: true }).fill(name);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await waitForSurface(page, "data");
  await page.locator(".nd-toast").filter({ hasText: /General SEM project draft active/i }).waitFor({ state: "visible", timeout: 15_000 });
}

async function importFixture(page, python, fixturePath, owner = {}) {
  await page.keyboard.press("Control+i");
  const dialog = page.getByRole("dialog", { name: "Import Data", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.locator('input[name="data-kind"][value="raw"]').check();
  await dialog.getByLabel(/^Missing-value markers/).fill("");
  const windowTitle = PACKAGED_MAIN_WINDOW_TITLE;
  const evidence = await withNativeDialog(page, {
    python,
    mode: "open",
    target: fixturePath,
    extensions: ["csv"],
    windowTitle,
    ownerPid: owner.candidatePid ?? null,
    ownerExecutable: owner.candidatePath ?? null,
  }, () => dialog.getByRole("button", { name: "Choose File…", exact: true }).click());
  await page.locator(".nd-data-table tbody tr").first().waitFor({ state: "visible", timeout: 30_000 });
  return evidence;
}

async function createEmptyModel(page, name) {
  const command = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^New Model/i });
  await command.waitFor({ state: "visible", timeout: 10_000 });
  await command.click();
  const dialog = page.getByRole("dialog", { name: "New Model", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByLabel("Name", { exact: true }).fill(name);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await waitForSurface(page, "model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
}

function modelInspector(page) { return page.locator("aside.nd-model-inspector"); }
async function inspectorTab(page, label) {
  const tab = modelInspector(page).getByRole("tab", { name: label, exact: true });
  await tab.waitFor({ state: "visible", timeout: 5_000 });
  if (await tab.getAttribute("aria-selected") !== "true") await tab.click();
}
async function clearSelection(page) {
  const pane = page.locator(".react-flow__pane");
  const box = await pane.boundingBox();
  if (!box) throw new Error("The model canvas did not expose bounds for selection clearing.");
  await pane.click({ position: { x: Math.max(8, box.width - 24), y: 24 } });
  await page.locator(".react-flow__node-latent.selected").waitFor({ state: "hidden", timeout: 5_000 }).catch(() => undefined);
}
async function clickIndicator(page, name) {
  const indicator = page.locator(".nd-variable-item").filter({ hasText: new RegExp(`^${name}$`) });
  await indicator.waitFor({ state: "visible", timeout: 10_000 });
  if (await indicator.count() !== 1) throw new Error(`Expected one ${name} indicator.`);
  await indicator.click();
}
async function renameSelected(page, name) {
  await inspectorTab(page, "Model");
  const inspector = modelInspector(page);
  const nameInput = inspector.getByLabel("Name", { exact: true });
  const shortInput = inspector.getByLabel("Short name", { exact: true });
  await nameInput.fill(name); await nameInput.press("Enter");
  await shortInput.fill(name); await shortInput.press("Enter");
  const expert = inspector.getByRole("button", { name: "Expert", exact: true });
  if (await expert.getAttribute("aria-pressed") !== "true") await expert.click();
  await inspectorTab(page, "Parameter");
  const representation = inspector.getByLabel("Representation", { exact: true });
  await representation.selectOption("composite");
  await inspector.getByText(/Composite confirmed\./).waitFor({ state: "visible", timeout: 5_000 });
}
function structuralPaths(page) { return page.locator('.react-flow__edge[data-id]:not([data-id^="measurement::"])'); }
async function createPath(page, nodes, sourceIndex, targetIndex, expected) {
  const command = page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ });
  await command.click();
  await nodes.nth(sourceIndex).dispatchEvent("click");
  await nodes.nth(targetIndex).dispatchEvent("click");
  await structuralPaths(page).nth(expected - 1).waitFor({ state: "attached", timeout: 10_000 });
  if (await structuralPaths(page).count() !== expected) throw new Error(`Expected ${expected} structural paths.`);
}
async function buildConstructs(page, definitions) {
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(page, definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    for (const indicator of definition.indicators.slice(1)) await clickIndicator(page, indicator);
    await renameSelected(page, definition.name);
    await clearSelection(page);
  }
  return nodes;
}
async function selectPath(page, edge) {
  const target = edge.locator(".react-flow__edge-interaction");
  await target.dispatchEvent("click");
  await edge.waitFor({ state: "attached", timeout: 10_000 });
}
async function addModerator(page, baseEdge, moderatorName) {
  await selectPath(page, baseEdge);
  const command = page.locator(".nd-commandbar button").filter({ hasText: /^Moderating Effect/i });
  if (!await command.isEnabled()) throw new Error(`Moderating Effect is unavailable for ${moderatorName}.`);
  await command.click();
  const dialog = page.getByRole("dialog", { name: "Create Moderating Effect", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  const moderator = dialog.locator("#nd-moderation-moderator");
  await moderator.selectOption({ label: moderatorName });
  const create = dialog.getByRole("button", { name: "Create moderating effect", exact: true });
  if (!await create.isEnabled()) throw new Error(`Moderation dialog rejected ${moderatorName}.`);
  await create.click();
  await dialog.waitFor({ state: "hidden", timeout: 10_000 });
}

async function buildRank0Model(page, family) {
  if (family === "mediation") {
    const nodes = await buildConstructs(page, [
      { name: "X", indicators: ["x1", "x2"] },
      { name: "M1", indicators: ["m11", "m12"] },
      { name: "M2", indicators: ["m21", "m22"] },
      { name: "Y", indicators: ["y1", "y2"] },
    ]);
    for (const [source, target, count] of [[0, 1, 1], [0, 2, 2], [1, 3, 3], [2, 3, 4], [0, 3, 5]]) {
      await createPath(page, nodes, source, target, count);
    }
    return { constructs: 4, indicators: 8, structuralPaths: 5, interactions: 0 };
  }
  const nodes = await buildConstructs(page, [
    { name: "X", indicators: ["x1", "x2"] },
    { name: "W", indicators: ["w1", "w2"] },
    { name: "Z", indicators: ["z1", "z2"] },
    { name: "Y", indicators: ["y1", "y2"] },
  ]);
  await createPath(page, nodes, 0, 3, 1);
  const baseId = await structuralPaths(page).first().getAttribute("data-id");
  if (!baseId) throw new Error("The X -> Y path has no stable identity.");
  const base = page.locator(`.react-flow__edge[data-id="${baseId}"]`);
  await addModerator(page, base, "W");
  await addModerator(page, base, "Z");
  const interactionNodes = page.locator('.react-flow__node-latent[data-id^="interaction-"]');
  await interactionNodes.nth(1).waitFor({ state: "visible", timeout: 10_000 });
  return {
    constructs: await nodes.count(),
    indicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths(page).count(),
    interactions: await interactionNodes.count(),
  };
}

async function openGeneralSem(page, requireStandardAccess = false) {
  const tab = page.locator("#nd-model-general-sem-labs-tab");
  await tab.waitFor({ state: "visible", timeout: 10_000 });
  await tab.click();
  const panel = page.locator("#nd-model-general-sem-labs-panel");
  await panel.waitFor({ state: "visible", timeout: 10_000 });
  const visibleText = compact(await panel.innerText());
  if (!visibleText.includes("Registry-authorized PLS-SEM estimation")) throw new Error(`General SEM workspace lacks Registry authority wording: ${visibleText}`);
  const labsChipCount = await tab.locator(".nd-experimental-chip").count();
  if (requireStandardAccess && (/\b(?:Labs|Experimental)\b/i.test(visibleText) || labsChipCount)) {
    throw new Error(`Standard General SEM workspace exposes non-Standard authority wording: ${visibleText}`);
  }
  if (!requireStandardAccess && labsChipCount === 0) throw new Error("Pre-promotion General SEM tab did not retain its Labs label.");
  return tab;
}

async function configureInference(page, bootstrap) {
  const checkbox = page.locator("#nd-general-sem-bootstrap");
  if (bootstrap !== await checkbox.isChecked()) await checkbox.setChecked(bootstrap);
  if (bootstrap) {
    await page.locator("#nd-general-sem-bootstrap-samples").fill("500");
    await page.locator("#nd-general-sem-confidence").fill("0.95");
  }
  await page.locator("#nd-general-sem-seed").fill("20260819");
  await page.locator("#nd-general-sem-workers").fill("1");
}

async function saveAndActivate(page, python, projectPath) {
  const button = page.getByRole("button", { name: "Save and activate project…", exact: true });
  if (!await button.isEnabled()) {
    throw new Error(`Save and activate remained blocked: ${compact(await page.locator("#nd-general-sem-preflight").textContent())}`);
  }
  const windowTitle = PACKAGED_MAIN_WINDOW_TITLE;
  const save = await withNativeDialog(page, {
    python, mode: "save", target: projectPath, extensions: ["qpls"], windowTitle,
  }, () => button.click());
  await page.getByText("General SEM (general_sem_v1)", { exact: true }).waitFor({ state: "visible", timeout: 30_000 });
  const calculate = page.locator("#nd-model-general-sem-labs-panel button.primary").filter({ hasText: /^Calculate/ });
  await calculate.waitFor({ state: "visible", timeout: 30_000 });
  try {
    await page.locator('#nd-model-general-sem-labs-panel button.primary:not([disabled])')
      .filter({ hasText: /^Calculate/ })
      .waitFor({ state: "visible", timeout: 30_000 });
  } catch {
    throw new Error(`Activated calculation remained disabled: ${JSON.stringify({
      title: await calculate.getAttribute("title"),
      panel: compact(await page.locator("#nd-model-general-sem-labs-panel").textContent()),
    })}`);
  }
  return save;
}

async function inferenceSettings(page) {
  const bootstrap = await page.locator("#nd-general-sem-bootstrap").isChecked();
  return {
    bootstrap,
    bootstrapSamples: bootstrap ? await page.locator("#nd-general-sem-bootstrap-samples").inputValue() : null,
    confidence: bootstrap ? await page.locator("#nd-general-sem-confidence").inputValue() : null,
    seed: await page.locator("#nd-general-sem-seed").inputValue(),
    workers: await page.locator("#nd-general-sem-workers").inputValue(),
  };
}

async function cancelAndVerify(page, start, python, projectPath) {
  const workspace = page.locator("#nd-model-general-sem-labs-panel");
  const settingsBefore = await inferenceSettings(page);
  const archiveBefore = await schema6ArchiveIdentity(python, projectPath);
  const visibleResultCountBefore = await workspace.locator(".nd-cbsem-v4-results").count();
  await start.click();
  const monitor = workspace.locator(".nd-cbsem-v4-monitor");
  await monitor.waitFor({ state: "visible", timeout: 20_000 });
  const cancel = page.getByRole("button", { name: "Cancel", exact: true });
  const state = monitor.locator(".nd-cbsem-v4-state");
  const stateBeforeCancel = compact(await state.textContent()).toLowerCase();
  if (stateBeforeCancel === "completed" || !await cancel.isVisible() || !await cancel.isEnabled()) {
    throw new Error("Calculation completed before a live cancellation request could be observed.");
  }
  const cancelStarted = performance.now();
  await cancel.click();
  let terminalState = "";
  while ((performance.now() - cancelStarted) <= 1_000) {
    terminalState = compact(await state.textContent()).toLowerCase();
    if (["cancelled", "completed", "failed"].includes(terminalState)) break;
    await page.waitForTimeout(10);
  }
  const terminalLatencySeconds = (performance.now() - cancelStarted) / 1_000;
  if (terminalState !== "cancelled") {
    throw new Error(`Cancellation did not reach terminal cancelled within 1.0s; observed ${terminalState || "non-terminal"} after ${terminalLatencySeconds.toFixed(6)}s.`);
  }
  if (terminalLatencySeconds > 1) {
    throw new Error(`Cancellation terminal latency ${terminalLatencySeconds.toFixed(6)}s exceeds 1.0s.`);
  }
  const visibleResultCountAfter = await workspace.locator(".nd-cbsem-v4-results").count();
  const committedResultActionCount = await workspace.getByRole("button", { name: "Save result to project", exact: true }).count();
  const archiveAfter = await schema6ArchiveIdentity(python, projectPath);
  const archiveUnchanged = archiveBefore.byte_length === archiveAfter.byte_length
    && archiveBefore.sha256 === archiveAfter.sha256
    && archiveBefore.canonical_result_attachment_count === archiveAfter.canonical_result_attachment_count;
  const noPartialVisibleResult = visibleResultCountAfter === visibleResultCountBefore;
  const noPartialCommittedResult = committedResultActionCount === 0
    && archiveAfter.canonical_result_attachment_count === archiveBefore.canonical_result_attachment_count;
  if (!noPartialVisibleResult || !noPartialCommittedResult) {
    throw new Error("Cancelled calculation exposed or committed a partial result.");
  }
  if (!archiveUnchanged) {
    throw new Error(`Cancelled calculation changed the schema-6 archive: ${JSON.stringify({ archiveBefore, archiveAfter })}`);
  }
  await monitor.getByRole("button", { name: "Clear terminal job", exact: true }).click();
  await monitor.waitFor({ state: "hidden", timeout: 10_000 });
  if (!await start.isEnabled()) throw new Error("The exact cancelled calculation setup was not retryable.");
  const settingsRetry = await inferenceSettings(page);
  const exactSameSettingsRetry = JSON.stringify(settingsRetry) === JSON.stringify(settingsBefore);
  if (!exactSameSettingsRetry) throw new Error("Cancelled calculation retry settings changed.");
  return {
    terminalLatencySeconds,
    terminalState,
    jobCompletedBeforeCancel: false,
    noPartialVisibleResult,
    noPartialCommittedResult,
    archiveUnchanged,
    exactSameSettingsRetry,
    visibleResultCountBefore,
    visibleResultCountAfter,
    committedResultActionCount,
    archiveBefore,
    archiveAfter,
    settingsBefore,
    settingsRetry,
  };
}

async function startAndWait(page, bootstrap, python, projectPath) {
  const workspace = page.locator("#nd-model-general-sem-labs-panel");
  const start = page.getByRole("button", { name: bootstrap ? /Calculate .*bootstrap|Calculate PLS effects/i : /Calculate .*point estimates|Calculate PLS effects/i });
  if (!await start.isEnabled()) throw new Error("General SEM calculation is not enabled after native preflight.");
  const cancellation = await cancelAndVerify(page, start, python, projectPath);
  await start.click();
  const result = workspace.locator(".nd-cbsem-v4-results");
  await result.waitFor({ state: "visible", timeout: 300_000 });
  const resultHeading = result.locator("#nd-cbsem-v4-results-heading");
  await resultHeading.waitFor({ state: "visible", timeout: 20_000 });
  if (!compact(await resultHeading.textContent())) throw new Error("Canonical result heading is empty.");
  await workspace.locator(".nd-canonical-export-v2").waitFor({ state: "visible", timeout: 20_000 });
  return cancellation;
}

async function canonicalIdentity(page) {
  const exportPanel = page.locator(".nd-canonical-export-v2");
  await exportPanel.getByText("Export identity", { exact: true }).click();
  const values = await exportPanel.locator("details dl > div").evaluateAll((rows) => Object.fromEntries(rows.map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const saved = compact(await page.locator(".nd-cbsem-v4-success").filter({ hasText: /Saved result/ }).textContent().catch(() => ""));
  const documentId = values.Document ?? saved.match(/Saved result (.+)\.$/)?.[1] ?? null;
  return {
    documentId,
    runId: values.Run ?? null,
    methodVersion: values.Method ?? null,
    datasetFingerprint: values["Dataset fingerprint"] ?? null,
    tableIds: await page.locator("[data-canonical-table-id]").evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-canonical-table-id"))),
    chartIds: await page.locator("[data-canonical-chart-id]").evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-canonical-chart-id"))),
  };
}

async function exerciseExportCancellation(page, python, evidenceDir) {
  const panel = page.locator(".nd-canonical-export-v2");
  const target = path.join(evidenceDir, "cancelled-canonical-result.csv");
  const button = panel.getByRole("button", { name: "Export CSV", exact: true });
  await button.waitFor({ state: "visible", timeout: 10_000 });
  if (!await button.isEnabled()) throw new Error("CSV export is unavailable for cancellation acceptance.");
  const publication = await withNativeDialog(page, {
    python,
    mode: "save-cancel",
    target,
    extensions: ["csv"],
    windowTitle: PACKAGED_MAIN_WINDOW_TITLE,
  }, () => button.click());
  const feedback = panel.locator(".nd-export-feedback.neutral").filter({
    hasText: "Export cancelled. No native file was published",
  });
  await feedback.waitFor({ state: "visible", timeout: 15_000 });
  const targetExists = await fs.stat(target).then(() => true).catch((error) => {
    if (error?.code === "ENOENT") return false;
    throw error;
  });
  if (targetExists
    || publication?.event !== "complete"
    || publication?.passed !== true
    || publication?.mode !== "save-cancel"
    || publication?.file?.exists !== false
    || publication?.file?.cancelledBeforePublication !== true
    || path.resolve(publication?.file?.path ?? "") !== path.resolve(target)) {
    throw new Error(`Cancelled native export did not prove zero publication: ${JSON.stringify({ publication, targetExists })}`);
  }
  return {
    format: "csv",
    destinationPath: target,
    nativeDialogCancelled: true,
    semanticReadbackCompleted: compact(await feedback.textContent()).toLowerCase().includes("semantic readback completed before the publication boundary."),
    destinationExistedAfter: false,
    noPartialFile: true,
    publication,
  };
}

async function exportAll(page, python, evidenceDir, identity) {
  const panel = page.locator(".nd-canonical-export-v2");
  const files = [];
  for (const format of ["csv", "xlsx", "html", "pdf", "svg", "png"]) {
    const target = path.join(evidenceDir, `canonical-result.${format}`);
    const button = panel.getByRole("button", { name: `Export ${format.toUpperCase()}`, exact: true });
    await button.waitFor({ state: "visible", timeout: 10_000 });
    if (!await button.isEnabled()) throw new Error(`Required ${format.toUpperCase()} export is unavailable for ${identity.documentId}.`);
    const publication = await withNativeDialog(page, {
      python,
      mode: "save",
      target,
      extensions: [format],
      windowTitle: PACKAGED_MAIN_WINDOW_TITLE,
    }, () => button.click());
    const file = await fileIdentity(target);
    if (file.size <= 0) throw new Error(`${format.toUpperCase()} export is empty.`);
    if (publication?.event !== "complete"
      || publication?.passed !== true
      || publication?.mode !== "save"
      || path.resolve(publication?.file?.path ?? "") !== path.resolve(target)
      || publication?.file?.size !== file.size
      || publication?.file?.sha256 !== file.sha256) {
      throw new Error(`Native ${format.toUpperCase()} publication receipt does not bind the final bytes: ${JSON.stringify({ publication, file })}`);
    }
    const feedback = panel.locator(".nd-export-feedback.success").filter({ hasText: target });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const feedbackText = compact(await feedback.textContent());
    if (!feedbackText.includes("Semantic readback passed before publication.")) {
      throw new Error(`Native ${format.toUpperCase()} publication did not expose the verified semantic receipt.`);
    }
    files.push({ format, ...file, publication, feedback: feedbackText });
  }
  return files;
}

async function appendVerifyClose(page) {
  const save = page.getByRole("button", { name: "Save result to project", exact: true });
  await save.click();
  const status = page.locator(".nd-cbsem-v4-success").filter({ hasText: /Saved result/ });
  await status.waitFor({ state: "visible", timeout: 30_000 });
  const documentId = compact(await status.textContent()).match(/Saved result (.+)\.$/)?.[1] ?? null;
  const reopen = page.getByRole("button", { name: "Reopen and verify", exact: true });
  await reopen.click();
  const close = page.getByRole("button", { name: "Close General SEM project", exact: true });
  await close.click({ timeout: 60_000 });
  await waitForSurface(page, "data", 20_000);
  return { documentId, closed: true };
}

async function executePrimary(page, args, trace) {
  const variant = VARIANTS[args["variant-id"]];
  const name = `Rank 0 ${args["package-kind"]} ${args["variant-id"]}`;
  const fixturePath = path.join(args["evidence-dir"], "rank0-input.csv");
  trace.fixture = await fixtureFor(args["variant-id"], fixturePath);
  await createGeneralSemDraft(page, name, args.requireStandardAccess);
  trace.steps.launch_offline = true;
  trace.steps.create_general_sem_project = true;
  trace.standardAccess = { labsPreference: !args.requireStandardAccess, labsChipCount: args.requireStandardAccess ? 0 : 1, projectMode: "general_sem_v1" };
  trace.import = await importFixture(page, args.python, fixturePath);
  await createEmptyModel(page, "Rank 0 General SEM model");
  await openGeneralSem(page, args.requireStandardAccess);
  const create = page.getByRole("button", { name: "Save and activate project…", exact: true });
  trace.invalidSetup = {
    saveAndActivateEnabled: await create.isEnabled(),
    diagnostics: compact(await page.locator("#nd-general-sem-preflight").textContent()),
  };
  if (trace.invalidSetup.saveAndActivateEnabled || !trace.invalidSetup.diagnostics) throw new Error("Invalid empty setup did not fail closed.");
  trace.steps.invalid_setup_fail_closed = true;
  await page.locator("#nd-model-canvas-tab").click();
  trace.model = await buildRank0Model(page, variant.family);
  await openGeneralSem(page, args.requireStandardAccess);
  await configureInference(page, variant.bootstrap);
  trace.steps.valid_setup = true;
  trace.saveAndActivate = await saveAndActivate(page, args.python, args["project-path"]);
  const beforeArchive = await fileIdentity(args["project-path"]);
  trace.cancellation = await startAndWait(page, variant.bootstrap, args.python, args["project-path"]);
  const identity = await canonicalIdentity(page);
  if (!identity.runId || !identity.methodVersion || !identity.datasetFingerprint) throw new Error(`Canonical result identity is incomplete: ${JSON.stringify(identity)}`);
  trace.identity = identity;
  trace.steps.execute_and_monitor = true;
  trace.steps.canonical_result_authority = true;
  if (!trace.cancellation) throw new Error("Rank 0 acceptance lacks calculation cancellation evidence.");
  trace.steps.cancel_retry_no_partial = true;
  const workflowOnly = args["workflow-only"] === "true";
  let exports = [];
  if (!workflowOnly) {
    trace.exportCancellation = {
      saveDialog: await exerciseExportCancellation(page, args.python, args["evidence-dir"]),
    };
    if (!trace.exportCancellation.saveDialog.semanticReadbackCompleted) {
      throw new Error("Cancelled export did not retain semantic pre-publication readback evidence.");
    }
    trace.steps.export_cancel_no_partial_file = true;
    exports = await exportAll(page, args.python, args["evidence-dir"], identity);
    for (const format of ["csv", "xlsx", "html", "pdf", "svg", "png"]) trace.steps[`export_${format}`] = true;
  }
  const persisted = await appendVerifyClose(page);
  if (persisted.documentId !== identity.documentId) throw new Error("The appended document identity differs from the displayed canonical result.");
  trace.steps.append_result = true;
  trace.steps.close_project = true;
  trace.projectArchiveBeforeRun = beforeArchive;
  trace.projectArchive = await fileIdentity(args["project-path"]);
  trace.identity.documentId = persisted.documentId;
  if (!workflowOnly) {
    trace.exportedFiles = exports;
    await writeJsonNew(path.join(args["evidence-dir"], "raw-exported-files.json"), {
      schema_version: 1,
      evidence_kind: "raw_exported_files",
      package_kind: args["package-kind"],
      variant_id: args["variant-id"],
      run_id: identity.runId,
      document_id: persisted.documentId,
      files: exports,
    });
  }
}

async function openExactProject(page, projectPath, requireStandardAccess = false, requireVerifiedResult = true) {
  const project = page.locator(".nd-window-project").filter({ hasText: /Rank 0/i });
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    await page.evaluate(({ target }) => {
      window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } }));
    }, { target: projectPath });
    if (await project.waitFor({ state: "visible", timeout: 10_000 }).then(() => true).catch(() => false)) break;
    if (attempt === 3) throw new Error("The fresh process did not open the exact saved General SEM project.");
  }
  await waitForSurface(page, "model", 30_000);
  await openGeneralSem(page, requireStandardAccess);
  if (requireVerifiedResult) {
    await page.getByText(/Verified project result/i).waitFor({ state: "visible", timeout: 30_000 });
  }
}

async function keyboardSnapshot(page) {
  await page.locator("#nd-model-general-sem-labs-tab").focus();
  const sequence = [];
  for (let index = 0; index < 24; index += 1) {
    await page.keyboard.press("Tab");
    sequence.push(await page.evaluate(() => {
      const active = document.activeElement;
      return {
        tag: active?.tagName?.toLowerCase() ?? null,
        id: active?.id || null,
        role: active?.getAttribute("role") || null,
        name: active?.getAttribute("aria-label") || active?.textContent?.replace(/\s+/g, " ").trim().slice(0, 120) || null,
      };
    }));
  }
  return {
    sequence,
    distinctTargets: new Set(sequence.map(({ tag, id, name }) => `${tag}:${id}:${name}`)).size,
    reachedInteractiveControl: sequence.some(({ tag }) => ["button", "input", "select", "summary"].includes(tag)),
  };
}

async function reopenMatrix(page, args, trace) {
  const expected = JSON.parse(await fs.readFile(args["identity-file"], "utf8"));
  await openExactProject(page, args["project-path"], args.requireStandardAccess);
  const identity = await canonicalIdentity(page);
  const scale = Number(args["scale-percent"]);
  const cells = [];
  for (const viewport of VIEWPORTS) {
    const resize = await setActualTauriClientViewport(page, viewport, `General SEM Rank 0 ${scale}% ${viewport.id}`);
    const snapshot = await page.evaluate(() => {
      const app = document.querySelector(".nd-app[data-native-desktop-shell='true']");
      const tables = [...document.querySelectorAll(".nd-cbsem-v4-table-wrap table")];
      const charts = [...document.querySelectorAll('.nd-canonical-chart[role="img"]')];
      return {
        origin: location.origin,
        tauriRuntime: Boolean(window.__TAURI_INTERNALS__),
        surface: app?.getAttribute("data-surface") ?? null,
        innerWidth,
        innerHeight,
        devicePixelRatio,
        documentNoHorizontalOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
        appNoHorizontalOverflow: Boolean(app && app.scrollWidth <= app.clientWidth + 1),
        tableCount: tables.length,
        accessibleTableCount: tables.filter((table) => Boolean(table.getAttribute("aria-labelledby") || table.querySelector("caption"))).length,
        chartCount: charts.length,
        accessibleChartCount: charts.filter((chart) => Boolean(chart.getAttribute("aria-label") || chart.getAttribute("aria-labelledby"))).length,
      };
    });
    const keyboard = await keyboardSnapshot(page);
    const passed = snapshot.origin === PACKAGED_TAURI_ORIGIN
      && snapshot.tauriRuntime
      && snapshot.surface === "model"
      && snapshot.innerWidth === viewport.width
      && snapshot.innerHeight === viewport.height
      && Math.abs(snapshot.devicePixelRatio - scale / 100) <= 0.08
      && snapshot.documentNoHorizontalOverflow
      && snapshot.appNoHorizontalOverflow
      && snapshot.tableCount > 0
      && snapshot.accessibleTableCount === snapshot.tableCount
      && snapshot.chartCount > 0
      && snapshot.accessibleChartCount === snapshot.chartCount
      && keyboard.distinctTargets >= 4
      && keyboard.reachedInteractiveControl;
    cells.push({ scale_percent: scale, viewport: viewport.id, resize, snapshot, keyboard, passed });
    if (!passed) throw new Error(`Viewport/accessibility cell failed: ${JSON.stringify(cells.at(-1))}`);
  }
  if (identity.runId !== expected.identity.runId || identity.documentId !== expected.identity.documentId) {
    throw new Error(`Fresh-process reopen identity mismatch: ${JSON.stringify({ expected: expected.identity, actual: identity })}`);
  }
  trace.identity = identity;
  trace.expectedIdentity = expected.identity;
  trace.projectArchive = await fileIdentity(args["project-path"]);
  trace.scalePercent = scale;
  trace.cells = cells;
  trace.freshProcessReopen = true;
  trace.keyboardNavigation = cells.every(({ keyboard }) => keyboard.reachedInteractiveControl);
  trace.accessibleTableAndChart = cells.every(({ snapshot }) => snapshot.tableCount > 0 && snapshot.accessibleTableCount === snapshot.tableCount && snapshot.chartCount > 0 && snapshot.accessibleChartCount === snapshot.chartCount);
  trace.viewportScaling = cells.every(({ passed }) => passed);
  const close = page.getByRole("button", { name: "Close General SEM project", exact: true });
  await close.click();
  await waitForSurface(page, "data", 20_000);
  trace.closeProject = true;
}

export {
  VARIANTS,
  addModerator,
  buildConstructs,
  cancelAndVerify,
  canonicalIdentity,
  configureInference,
  createEmptyModel,
  createGeneralSemDraft,
  createPath,
  importFixture,
  openExactProject,
  openGeneralSem,
  saveAndActivate,
  startAndWait,
  structuralPaths,
  waitForSurface,
};

export async function runPackagedAcceptanceMain(argv = process.argv.slice(2)) {
const args = parseArgs(argv);
args.requireStandardAccess = args["require-standard-access"] === "true";
const evidenceDir = path.resolve(args["evidence-dir"]);
const projectPath = path.resolve(args["project-path"]);
if (!evidenceDir.startsWith(`${path.resolve(RESULTS_ROOT)}${path.sep}`) || !projectPath.startsWith(`${path.resolve(RESULTS_ROOT)}${path.sep}`)) {
  throw new Error("Evidence and project paths must remain below validation/results.");
}
await fs.mkdir(evidenceDir, { recursive: true });
args["evidence-dir"] = evidenceDir;
args["project-path"] = projectPath;
const trace = {
  schema_version: 1,
  evidence_kind: args.phase === "execute" ? "general_sem_rank0_primary_run" : "general_sem_rank0_fresh_reopen",
  package_kind: args["package-kind"],
  variant_id: args["variant-id"],
  phase: args.phase,
  process_id: Number(args["process-id"] ?? 0) || null,
  started_at_utc: new Date().toISOString(),
  steps: {},
  console_errors: [],
  failures: [],
};

let browser;
let page;
let offline;
try {
  ({ browser, page } = await connectToSingleQuickPlsPage({ chromium, endpoint: args.endpoint }));
  await page.evaluate(({ enabled }) => localStorage.setItem("quickpls:native-ui-preferences:v1", JSON.stringify({ experimentalLabsEnabled: enabled })), { enabled: !args.requireStandardAccess });
  await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  offline = observeFunctionalOfflineRequests(page);
  page.on("pageerror", (error) => trace.console_errors.push({ type: "pageerror", message: error.message, stack: error.stack ?? null }));
  page.on("console", (message) => { if (message.type() === "error") trace.console_errors.push({ type: "console", message: message.text() }); });
  if (args.phase === "execute") await executePrimary(page, args, trace);
  else await reopenMatrix(page, args, trace);
  trace.offline = offline.summary();
  if (!trace.offline.passed) throw new Error(`Functional-offline boundary failed: ${JSON.stringify(trace.offline)}`);
  if (trace.console_errors.length) throw new Error(`Packaged console errors: ${JSON.stringify(trace.console_errors)}`);
} catch (error) {
  trace.failures.push(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
} finally {
  offline?.stop();
  trace.completed_at_utc = new Date().toISOString();
  trace.passed = trace.failures.length === 0 && trace.console_errors.length === 0;
  const fileName = args.phase === "execute" ? "raw-run-trace.json" : `raw-reopen-${args["scale-percent"]}.json`;
  await writeJsonNew(path.join(evidenceDir, fileName), trace).catch(async (error) => {
    console.error(`Could not write raw General SEM evidence: ${error}`);
    process.exitCode = 1;
  });
  await browser?.close().catch(() => undefined);
}

console.log(JSON.stringify({
  passed: trace.passed,
  phase: args.phase,
  package_kind: args["package-kind"],
  variant_id: args["variant-id"],
  evidence_dir: evidenceDir,
  failures: trace.failures,
}, null, 2));
return trace;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runPackagedAcceptanceMain();
}
