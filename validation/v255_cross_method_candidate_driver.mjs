#!/usr/bin/env node
/** Attach-only QuickPLS 2.55 cross-method candidate driver.
 *
 * The PowerShell wrapper owns every process.  This file attaches to one exact
 * loopback CDP endpoint, drives real native commands/dialogs, observes only
 * product DOM/file outcomes, and never starts, closes, or terminates QuickPLS.
 */

import crypto from "node:crypto";
import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import {
  PACKAGED_TAURI_ORIGIN,
  connectToSingleQuickPlsPage,
  observeFunctionalOfflineRequests,
} from "./v247_cdp_package_helpers.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const FILE_HELPER = path.join(ROOT, "validation", "windows_native_owned_file_dialog.py");
const SUITE_ID = "quickpls_v255_cross_method_candidate_driver_v1";
const MANIFEST_SUITE_ID = "quickpls_v255_cross_method_case_manifest_v1";
const PHASES = new Set(["imports", "exports", "archives", "legacy_reopen", "autosave_seed", "autosave_recover", "unsaved_close_seed", "dpi_process"]);
const DPI_WAIVER_METADATA = Object.freeze({
  waiver_authority: "product_owner",
  waiver_date: "2026-08-22",
  reason: "product owner explicitly authorized ignoring the 200 percent scaling requirement",
});

function assert(value, message) { if (!value) throw new Error(message); }
function compact(value) { return String(value ?? "").replace(/\s+/g, " ").trim(); }
function sha256(value) { return crypto.createHash("sha256").update(value).digest("hex"); }
async function fileSha256(file) { return sha256(await fs.readFile(file)); }
async function exists(file) { return fs.stat(file).then(() => true, () => false); }
function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}
async function readJson(file) {
  const value = JSON.parse(await fs.readFile(file, "utf8"));
  assert(value && typeof value === "object" && !Array.isArray(value), `JSON root must be an object: ${file}`);
  return value;
}
async function writeJsonNew(file, value) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  await fs.writeFile(file, `${JSON.stringify(value, null, 2)}\n`, { flag: "wx" });
}
function parseArgs(argv) {
  const allowed = new Set(["phase", "endpoint", "evidence-dir", "manifest", "fixture-report", "candidate-path", "candidate-sha256", "candidate-pid", "source-commit", "release-report-sha256", "python", "project-path", "effective-dpi", "waive-actual-windows-200-percent-scaling"]);
  const args = {};
  for (let index = 0; index < argv.length; index += 2) {
    const token = argv[index];
    assert(token?.startsWith("--") && argv[index + 1] && !argv[index + 1].startsWith("--"), `Invalid argument near ${token ?? "end"}.`);
    const key = token.slice(2);
    assert(allowed.has(key) && args[key] === undefined, `Unknown or duplicate argument: --${key}`);
    args[key] = argv[index + 1];
  }
  for (const key of ["phase", "endpoint", "evidence-dir", "manifest", "fixture-report", "candidate-path", "candidate-sha256", "candidate-pid", "source-commit", "release-report-sha256", "python"]) assert(args[key], `--${key} is required.`);
  assert(PHASES.has(args.phase), `Unsupported --phase: ${args.phase}`);
  assert(/^[0-9a-f]{64}$/iu.test(args["candidate-sha256"]), "Candidate SHA-256 is invalid.");
  assert(/^[0-9a-f]{40}$/u.test(args["source-commit"]), "Source commit is invalid.");
  assert(/^[0-9a-f]{64}$/iu.test(args["release-report-sha256"]), "Release report SHA-256 is invalid.");
  assert(/^\d+$/u.test(args["candidate-pid"]) && Number(args["candidate-pid"]) > 0, "Candidate PID is invalid.");
  assert(args["waive-actual-windows-200-percent-scaling"] === undefined || args["waive-actual-windows-200-percent-scaling"] === "true", "DPI waiver argument must be the exact value true.");
  assert(args["waive-actual-windows-200-percent-scaling"] === undefined || args.phase === "dpi_process", "The DPI waiver is valid only for the dpi_process phase.");
  const parsed = new URL(args.endpoint);
  assert(parsed.protocol === "http:" && parsed.hostname === "127.0.0.1" && parsed.port, "Endpoint must be explicit HTTP loopback 127.0.0.1.");
  return {
    ...args,
    evidenceDir: path.resolve(args["evidence-dir"]),
    manifest: path.resolve(args.manifest),
    fixtureReport: path.resolve(args["fixture-report"]),
    candidatePath: path.resolve(args["candidate-path"]),
    candidatePid: Number(args["candidate-pid"]),
    python: path.resolve(args.python),
    projectPath: args["project-path"] ? path.resolve(args["project-path"]) : null,
    effectiveDpi: args["effective-dpi"] ? Number(args["effective-dpi"]) : null,
    waiveActualWindows200PercentScaling: args["waive-actual-windows-200-percent-scaling"] === "true",
  };
}

function startFileHelper(args, { mode, target, extension, windowTitle }) {
  assert(typeof windowTitle === "string" && windowTitle.startsWith("QuickPLS"), "Native file helper requires the exact current QuickPLS title.");
  const child = spawn(args.python, [
    FILE_HELPER, "--mode", mode, "--target", target, "--allowed-root", path.dirname(target),
    "--window-title", windowTitle, "--extension", extension, "--timeout-seconds", "90",
    "--owner-pid", String(args.candidatePid), "--owner-executable", args.candidatePath,
  ], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  let buffer = "";
  let stderr = "";
  let readyResolve;
  let completeResolve;
  let readyDone = false;
  let completeDone = false;
  const ready = new Promise((resolve) => { readyResolve = resolve; });
  const complete = new Promise((resolve) => { completeResolve = resolve; });
  const settle = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      if (event.event === "ready" && !readyDone) { readyDone = true; readyResolve(event); }
      if (event.event === "complete" && !completeDone) {
        if (!readyDone) { readyDone = true; readyResolve(event); }
        completeDone = true; completeResolve({ ...event, stderr: compact(stderr) });
      }
    } catch (error) {
      const event = { event: "complete", passed: false, error: String(error), line };
      if (!readyDone) { readyDone = true; readyResolve(event); }
      if (!completeDone) { completeDone = true; completeResolve(event); }
    }
  };
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    buffer += chunk;
    const lines = buffer.split(/\r?\n/u);
    buffer = lines.pop() ?? "";
    lines.forEach(settle);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.on("error", (error) => settle(JSON.stringify({ event: "complete", passed: false, error: error.message })));
  child.on("close", (code) => {
    if (buffer.trim()) settle(buffer);
    if (!completeDone) settle(JSON.stringify({ event: "complete", passed: false, error: `helper exited ${code}`, stderr: compact(stderr) }));
  });
  return { ready, complete, stop: () => { if (!child.killed) child.kill(); } };
}

async function screenshot(page, evidenceDir, stem) {
  const target = path.join(evidenceDir, `${stem}.png`);
  assert(!await exists(target), `Screenshot target already exists: ${target}`);
  await page.screenshot({ path: target, fullPage: false });
  const bytes = await fs.readFile(target);
  assert(bytes.length > 1000 && bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])), `Invalid PNG: ${target}`);
  return { path: target, sha256: sha256(bytes) };
}

async function waitToast(page, title) {
  const toast = page.locator(".nd-toast", { has: page.locator("strong", { hasText: title }) }).last();
  await toast.waitFor({ state: "visible", timeout: 20_000 });
  return compact(await toast.innerText());
}

async function importFixture(page, args, fixture, entry) {
  const target = path.resolve(fixture.path);
  const extension = path.extname(target).slice(1);
  assert(inside(path.dirname(args.fixtureReport), target) && await exists(target), `Import fixture is unavailable: ${target}`);
  const helper = startFileHelper(args, { mode: "open", target, extension, windowTitle: await page.title() });
  try {
    const ready = await helper.ready;
    assert(ready.passed === true && ready.mainWindow?.pid === args.candidatePid, `File helper did not bind to PID ${args.candidatePid}.`);
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:import-data", { detail: { dataKind: "raw", sampleSize: null, missingMarkers: ["NA", "N/A", "."] } })));
    const completed = await helper.complete;
    assert(completed.passed === true && completed.mainWindow?.pid === args.candidatePid && completed.file?.sha256 === fixture.sha256, `Native ${entry.format} file selection failed.`);
  } finally { helper.stop(); }
  await waitToast(page, "Dataset imported");
  const observed = await page.evaluate(({ expectedName, format }) => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
    const table = document.querySelector('[role="grid"][aria-label^="Data table for "]');
    const headers = [...(table?.querySelectorAll("thead th[data-native-variable]") ?? [])].map((node) => clean(node.getAttribute("data-native-variable")));
    const rows = [...(table?.querySelectorAll("tbody tr") ?? [])].map((row) => [...row.querySelectorAll("td:not(.row-index)")].map((cell) => clean(cell.textContent)));
    const active = clean(document.querySelector('[data-native-dataset="active"] span')?.textContent);
    const missingTerm = [...document.querySelectorAll("dt")].find((node) => clean(node.textContent) === "Missing values");
    const missingText = clean(missingTerm?.nextElementSibling?.textContent);
    return { format, dataset_name: active, columns: headers, rows: rows.length, missing: /^\d+$/u.test(missingText) ? Number(missingText) : null, first_row: rows[0] ?? [], last_row: rows.at(-1) ?? [], expected_name_visible: active === expectedName };
  }, { expectedName: entry.expected.dataset_name, format: entry.format });
  delete observed.expected_name_visible;
  assert(JSON.stringify(observed) === JSON.stringify(entry.expected), `${entry.id} observation mismatch: ${JSON.stringify(observed)}`);
  return { id: entry.id, expected: entry.expected, observed, screenshot: await screenshot(page, args.evidenceDir, `import-${entry.format.toLowerCase().replace(/[^a-z0-9]+/gu, "-")}`) };
}

function semanticExport(format, bytes) {
  if (format === "CSV") return bytes.toString("utf8").includes(",") && bytes.toString("utf8").split(/\r?\n/u).length > 2;
  if (format === "XLSX") return bytes.subarray(0, 2).toString("binary") === "PK";
  if (format === "HTML") return /<(?:!doctype html|html|table)\b/iu.test(bytes.toString("utf8"));
  if (format === "PDF") return bytes.subarray(0, 5).toString("ascii") === "%PDF-";
  if (format === "SVG") return /<svg\b/iu.test(bytes.toString("utf8"));
  if (format === "PNG") return bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
  return false;
}

async function openProject(page, target, toastTitle = null) {
  await page.evaluate((projectPath) => window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: projectPath } })), target);
  if (toastTitle) return waitToast(page, toastTitle);
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 20_000 });
  await page.waitForTimeout(800);
  return null;
}

async function exportCases(page, args, manifest, fixtures) {
  const archive = fixtures.get("export_schema6");
  assert(archive, "Export schema-6 fixture is missing.");
  await openProject(page, path.resolve(archive.path), "Project opened");
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "results" } })));
  await page.locator(".nd-canonical-export-v2").waitFor({ state: "visible", timeout: 25_000 });
  assert(await page.locator("#nd-canonical-export-v2-chart option").count() > 0, "Export fixture has no chart for SVG/PNG.");
  const records = [];
  for (const entry of manifest.cases.filter((item) => item.phase === "exports" && item.operation === "export_result")) {
    const target = path.join(args.evidenceDir, `quickpls-v255-export.${entry.format.toLowerCase() === "xlsx" ? "xlsx" : entry.format.toLowerCase()}`);
    const extension = path.extname(target).slice(1);
    assert(!await exists(target), `Export target already exists: ${target}`);
    const helper = startFileHelper(args, { mode: "save", target, extension, windowTitle: await page.title() });
    try {
      const ready = await helper.ready;
      assert(ready.passed === true && ready.mainWindow?.pid === args.candidatePid, "Save helper failed exact PID binding.");
      await page.getByRole("button", { name: `Export ${entry.format}`, exact: true }).click();
      const completed = await helper.complete;
      assert(completed.passed === true && completed.file?.path === target && completed.mainWindow?.pid === args.candidatePid, `${entry.format} native save failed.`);
    } finally { helper.stop(); }
    const feedback = page.locator(".nd-export-feedback.success");
    await feedback.waitFor({ state: "visible", timeout: 20_000 });
    assert(/Semantic readback passed before publication\./u.test(await feedback.innerText()), `${entry.format} semantic-readback feedback is absent.`);
    await feedback.scrollIntoViewIfNeeded();
    const bytes = await fs.readFile(target);
    const observed = { format: entry.format, extension: path.extname(target), published: bytes.length > 20, semantic_readback: semanticExport(entry.format, bytes) };
    assert(JSON.stringify(observed) === JSON.stringify(entry.expected), `${entry.id} observation mismatch: ${JSON.stringify(observed)}`);
    const capture = await screenshot(page, args.evidenceDir, `export-${entry.format.toLowerCase()}`);
    assert(entry.format !== "PNG" || sha256(bytes) !== capture.sha256, "The exported PNG was reused as its UI evidence screenshot.");
    records.push({ id: entry.id, expected: entry.expected, observed, artifact: { path: target, bytes: bytes.length, sha256: sha256(bytes) }, screenshot: capture });
  }
  return records;
}

async function importsPhase(page, args, manifest, fixturePayload) {
  const fixtures = new Map(fixturePayload.files.map((row) => [row.role, row]));
  const records = [];
  for (const entry of manifest.cases.filter((item) => item.phase === "imports" && item.operation === "import_dataset")) {
    const fixture = fixtures.get(entry.fixture_role);
    assert(fixture, `Missing fixture role ${entry.fixture_role}.`);
    records.push(await importFixture(page, args, fixture, entry));
  }
  return records;
}

async function archivesPhase(page, args, manifest, fixturePayload) {
  const fixtures = new Map(fixturePayload.files.map((row) => [row.role, row]));
  const records = [];

  const legacy = manifest.cases.find((item) => item.id.endsWith("legacy migration"));
  const legacyFixture = fixtures.get(legacy.fixture_role);
  const legacyBefore = await fileSha256(legacyFixture.path);
  await openProject(page, path.resolve(legacyFixture.path), "Project upgrade pending");
  const mutationApplied = await page.evaluate(() => new Promise((resolve) => {
    const detail = { mutation: { kind: "create_model", name: "Legacy migration model" }, resolve: () => resolve(true), reject: (reason) => resolve({ error: String(reason) }) };
    window.dispatchEvent(new CustomEvent("quickpls:mutate-project-explorer", { detail }));
    window.setTimeout(() => resolve({ error: "mutation timeout" }), 10_000);
  }));
  assert(mutationApplied === true, `Legacy supported mutation failed: ${JSON.stringify(mutationApplied)}`);
  await waitToast(page, "Models updated");
  await page.waitForFunction(() => document.title.endsWith(" *"), null, { timeout: 10_000 });
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:save-project")));
  await waitToast(page, "Project saved");
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 20_000 });
  const backup = `${legacyFixture.path}.bak`;
  assert(await exists(backup) && await fileSha256(backup) === legacyBefore, "Legacy explicit save did not preserve the original archive as an exact backup.");
  assert(await fileSha256(legacyFixture.path) !== legacyBefore, "Legacy explicit save did not publish an upgraded archive.");
  records.push({ kind: "legacy_upgrade_seed", source_schema: 4, supported_mutation: "create_model", model_name: "Legacy migration model", original_sha256: legacyBefore, upgraded_sha256: await fileSha256(legacyFixture.path), backup_path: backup, backup_sha256: await fileSha256(backup), screenshot: await screenshot(page, args.evidenceDir, "persistence-legacy-upgrade-save") });

  const future = manifest.cases.find((item) => item.id.endsWith("future read-only archive"));
  const futureFixture = fixtures.get(future.fixture_role);
  const futureToast = await openProject(page, path.resolve(futureFixture.path), "Project opened read-only");
  const blocked = await page.evaluate(() => new Promise((resolve) => {
    const detail = { mutation: { kind: "create_model", name: "Forbidden future mutation" }, resolve: () => resolve(false), reject: () => resolve(true) };
    window.dispatchEvent(new CustomEvent("quickpls:mutate-project-explorer", { detail }));
    window.setTimeout(() => resolve(false), 5000);
  }));
  await waitToast(page, "Read-only project");
  const futureObserved = { toast: "Project opened read-only", source_schema: 7, read_only: /schema 7/iu.test(futureToast), save_mutation_blocked: blocked === true };
  assert(JSON.stringify(futureObserved) === JSON.stringify(future.expected), `${future.id} observation mismatch.`);
  records.push({ id: future.id, expected: future.expected, observed: futureObserved, screenshot: await screenshot(page, args.evidenceDir, "persistence-future-read-only") });
  return records;
}

async function legacyReopen(page, args) {
  assert(args.projectPath, "legacy_reopen requires --project-path.");
  await openProject(page, args.projectPath, "Project opened");
  await page.waitForFunction(() => document.body.innerText.includes("Legacy migration model"), null, { timeout: 20_000 });
  const writable = await page.evaluate(() => new Promise((resolve) => {
    const detail = { mutation: { kind: "create_model", name: "Legacy reopen writability probe" }, resolve: () => resolve(true), reject: () => resolve(false) };
    window.dispatchEvent(new CustomEvent("quickpls:mutate-project-explorer", { detail }));
    window.setTimeout(() => resolve(false), 10_000);
  }));
  assert(writable === true, "Freshly reopened upgraded legacy project is not writable.");
  await waitToast(page, "Models updated");
  return [{ kind: "legacy_reopen", fresh_reopen: true, model_name_visible: true, writable_after_reopen: true, screenshot: await screenshot(page, args.evidenceDir, "persistence-legacy-fresh-reopen") }];
}

async function importForDirtyProject(page, args, fixturePayload) {
  const fixture = fixturePayload.files.find((row) => row.role === "import_csv");
  assert(fixture, "CSV fixture is missing.");
  const helper = startFileHelper(args, { mode: "open", target: path.resolve(fixture.path), extension: "csv", windowTitle: await page.title() });
  try {
    const ready = await helper.ready;
    assert(ready.passed === true, "CSV dirty-project helper did not become ready.");
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:import-data", { detail: { dataKind: "raw", sampleSize: null, missingMarkers: ["NA"] } })));
    const completed = await helper.complete;
    assert(completed.passed === true && completed.mainWindow?.pid === args.candidatePid, "CSV dirty-project import failed.");
  } finally { helper.stop(); }
  await waitToast(page, "Dataset imported");
  await page.waitForFunction(() => document.title.endsWith(" *"), null, { timeout: 20_000 });
}

async function autosaveSeed(page, args, fixturePayload) {
  assert(args.projectPath, "autosave_seed requires --project-path.");
  await openProject(page, args.projectPath, "Project opened");
  await importForDirtyProject(page, args, fixturePayload);
  const autosave = `${args.projectPath}.autosave`;
  const deadline = Date.now() + 25_000;
  while (Date.now() < deadline && (!await exists(autosave) || (await fs.stat(autosave)).size === 0)) await new Promise((resolve) => setTimeout(resolve, 250));
  assert(await exists(autosave) && (await fs.stat(autosave)).size > 0, "Autosave was not created after a real dataset mutation.");
  return [{ kind: "autosave_seed", project_path: args.projectPath, project_sha256: await fileSha256(args.projectPath), autosave_path: autosave, autosave_sha256: await fileSha256(autosave), dirty_title: await page.title(), screenshot: await screenshot(page, args.evidenceDir, "autosave-seed") }];
}

async function autosaveRecover(page, args) {
  assert(args.projectPath, "autosave_recover requires --project-path.");
  const toast = await openProject(page, args.projectPath, "Project recovered");
  const dataset = compact(await page.locator('[data-native-dataset="active"] span').innerText());
  assert(dataset === "quickpls-v255-import.csv", `Recovered dataset is ${dataset}.`);
  return [{ kind: "autosave_recover", project_path: args.projectPath, recovered_toast: /Project recovered/u.test(toast), recovered_dataset: dataset, screenshot: await screenshot(page, args.evidenceDir, "autosave-recovered") }];
}

async function unsavedCloseSeed(page, args, fixturePayload) {
  assert(args.projectPath, "unsaved_close_seed requires --project-path.");
  await openProject(page, args.projectPath, "Project opened");
  await importForDirtyProject(page, args, fixturePayload);
  return [{ kind: "unsaved_close_seed", project_path: args.projectPath, dirty_title: await page.title(), screenshot: await screenshot(page, args.evidenceDir, "unsaved-close-dirty") }];
}

async function dpiProcess(page, args, offline) {
  if (args.waiveActualWindows200PercentScaling) {
    assert(Number.isInteger(args.effectiveDpi) && args.effectiveDpi > 0 && args.effectiveDpi !== 192, `Waived DPI evidence must retain a positive actual non-192 DPI (observed ${args.effectiveDpi}).`);
  } else {
    assert(args.effectiveDpi === 192, `Wrapper did not prove effective DPI 192 (observed ${args.effectiveDpi}).`);
  }
  const browserState = await page.evaluate(() => ({ device_pixel_ratio: window.devicePixelRatio, origin: window.location.origin, native_shell: Boolean(document.querySelector('.nd-app[data-native-desktop-shell="true"]')) }));
  assert(Number.isFinite(browserState.device_pixel_ratio) && browserState.device_pixel_ratio > 0 && browserState.origin === PACKAGED_TAURI_ORIGIN && browserState.native_shell, `DPI/process browser state is invalid: ${JSON.stringify(browserState)}`);
  const expectedDevicePixelRatio = args.effectiveDpi / 96;
  assert(Math.abs(browserState.device_pixel_ratio - expectedDevicePixelRatio) <= 0.02, `Browser DPR ${browserState.device_pixel_ratio} is inconsistent with native DPI ${args.effectiveDpi} (expected ${expectedDevicePixelRatio}).`);
  if (!args.waiveActualWindows200PercentScaling) assert(browserState.device_pixel_ratio === 2, `Actual Windows 200 percent evidence requires DPR 2 (observed ${browserState.device_pixel_ratio}).`);
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:open-demo-project", { detail: { sampleId: "corporate_reputation" } })));
  await waitToast(page, "Project opened");
  const dpiScreenshot = await screenshot(page, args.evidenceDir, args.waiveActualWindows200PercentScaling ? `waived-windows-scaling-actual-dpi-${args.effectiveDpi}` : "actual-windows-200-percent");
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "home" } })));
  await page.waitForTimeout(300);
  const cdpScreenshot = await screenshot(page, args.evidenceDir, "isolated-local-cdp");
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "data" } })));
  await page.waitForTimeout(300);
  const cleanupScreenshot = await screenshot(page, args.evidenceDir, "pid-scoped-cleanup-before-stop");
  const offlineSummary = offline.summary();
  return [{
    kind: "dpi_process",
    scaling_requirement_status: args.waiveActualWindows200PercentScaling ? "waived" : "passed",
    waiver: args.waiveActualWindows200PercentScaling ? DPI_WAIVER_METADATA : null,
    effective_dpi: args.effectiveDpi,
    browser: browserState,
    functional_network_requests: offlineSummary.externalRequestCount,
    screenshots: { dpi: dpiScreenshot, cdp: cdpScreenshot, cleanup: cleanupScreenshot },
  }];
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  assert(!await exists(args.evidenceDir), `Evidence directory must be new: ${args.evidenceDir}`);
  await fs.mkdir(args.evidenceDir, { recursive: true });
  const manifest = await readJson(args.manifest);
  const fixturePayload = await readJson(args.fixtureReport);
  assert(manifest.schema_version === 1 && manifest.suite_id === MANIFEST_SUITE_ID && manifest.target_release === "2.55.0" && manifest.status === "ready_for_collection", "Cross-method manifest identity is invalid.");
  assert(fixturePayload.schema_version === 1 && fixturePayload.suite_id === "quickpls_v255_cross_method_fixture_builder_v1" && fixturePayload.passed === true, "Fixture report is invalid.");
  assert(await fileSha256(args.candidatePath) === args["candidate-sha256"].toLowerCase(), "Candidate bytes changed before driver attach.");
  const { page, enumeratedPages } = await connectToSingleQuickPlsPage({ chromium, endpoint: args.endpoint });
  const offline = observeFunctionalOfflineRequests(page);
  let records;
  let offlineSummary;
  try {
    records = args.phase === "imports" ? await importsPhase(page, args, manifest, fixturePayload)
      : args.phase === "exports" ? await exportCases(page, args, manifest, new Map(fixturePayload.files.map((row) => [row.role, row])))
        : args.phase === "archives" ? await archivesPhase(page, args, manifest, fixturePayload)
          : args.phase === "legacy_reopen" ? await legacyReopen(page, args)
      : args.phase === "autosave_seed" ? await autosaveSeed(page, args, fixturePayload)
        : args.phase === "autosave_recover" ? await autosaveRecover(page, args)
          : args.phase === "unsaved_close_seed" ? await unsavedCloseSeed(page, args, fixturePayload)
            : await dpiProcess(page, args, offline);
    offlineSummary = offline.summary();
    assert(offlineSummary.passed === true && offlineSummary.externalRequestCount === 0, `Functional network requests escaped offline policy: ${JSON.stringify(offlineSummary)}`);
    const screenshots = records.flatMap((record) => record.screenshot ? [record.screenshot] : Object.values(record.screenshots ?? {}));
    assert(new Set(screenshots.map((item) => item.sha256)).size === screenshots.length, "Phase screenshots are not byte-unique.");
  } finally {
    offline.stop();
  }
  const report = {
    schema_version: 1,
    suite_id: SUITE_ID,
    target_release: "2.55.0",
    passed: true,
    phase: args.phase,
    source_commit: args["source-commit"],
    release_artifact_report_sha256: args["release-report-sha256"].toUpperCase(),
    candidate: { role: "portable", path: args.candidatePath, sha256: args["candidate-sha256"].toUpperCase(), pid: args.candidatePid },
    endpoint: args.endpoint,
    quickpls_page_count: enumeratedPages.filter((entry) => entry.shellVisible && entry.tauriRuntime).length,
    manifest: { path: args.manifest, sha256: await fileSha256(args.manifest) },
    fixture_report: { path: args.fixtureReport, sha256: await fileSha256(args.fixtureReport) },
    offline: offlineSummary,
    records,
    failures: [],
  };
  await writeJsonNew(path.join(args.evidenceDir, `v255_cross_method_${args.phase}.json`), report);
}

try {
  await main();
  await new Promise((resolve, reject) => {
    process.stdout.write(`${JSON.stringify({ passed: true })}\n`, (error) => {
      if (error) reject(error);
      else resolve();
    });
  });
  // The wrapper, not this attach-only client, owns and terminates QuickPLS.
  process.exit(0);
} catch (error) {
  await new Promise((resolve) => {
    process.stderr.write(`${error?.stack ?? error}\n`, resolve);
  });
  process.exit(1);
}
