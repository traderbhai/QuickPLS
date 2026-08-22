#!/usr/bin/env node
/**
 * QuickPLS 2.55 post-hoc technical minimum sample-size packaged journey.
 *
 * This source-only driver attaches to one already-launched, isolated packaged
 * Tauri WebView2 CDP endpoint. The wrapper owns process creation, exact-PID
 * checks, and termination. This file deliberately does not import
 * child_process and never closes a page, context, browser, or application.
 *
 * Execute:
 *   node validation/v255_posthoc_minimum_sample_packaged_smoke.mjs \
 *     --phase execute \
 *     --endpoint http://127.0.0.1:<isolated-port> \
 *     --evidence-dir validation/results/<new-candidate-directory>
 *
 * Fresh-process reopen:
 *   node validation/v255_posthoc_minimum_sample_packaged_smoke.mjs \
 *     --phase reopen \
 *     --endpoint http://127.0.0.1:<fresh-isolated-port> \
 *     --evidence-dir validation/results/<same-candidate-directory>
 *
 * Optional:
 *   --inventory validation/v255_reusable_archive_inventory.json
 *   --timeout-ms 300000
 *   --bootstrap-samples 1000
 *   --seed 20260822
 *   --workers 2
 */

import crypto from "node:crypto";
import { constants as fsConstants, createReadStream } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import {
  PACKAGED_TAURI_ORIGIN,
  enumerateQuickPlsCdpPages,
} from "./v247_cdp_package_helpers.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS_ROOT = path.join(ROOT, "validation", "results");
const DEFAULT_INVENTORY = path.join(ROOT, "validation", "v255_reusable_archive_inventory.json");
const POSTHOC_KIND = "pls_posthoc_technical_minimum_sample_size";
const SOURCE_KIND = "pls_bootstrap";
const TARGET_RELEASE = "2.55.0";
const TARGET_ARCHIVE_NAME = "quickpls-v255-posthoc-minimum-sample.qpls";
const EXECUTE_RECEIPT_NAME = "v255_posthoc_minimum_sample_packaged_smoke.json";
const REOPEN_RECEIPT_NAME = "v255_posthoc_minimum_sample_reopen.json";
const RENDERER_ERROR_SETTLE_MS = 250;
const EXPECTED_POSTHOC_RESULT_TITLE = "Post-hoc minimum sample size";
const EXPECTED_POSTHOC_RUN_LABEL = /Post-hoc Technical Minimum Sample Size/i;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function compact(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function slash(value) {
  return String(value).split(path.sep).join("/");
}

function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === ""
    || (!relative.startsWith(".." + path.sep) && relative !== ".." && !path.isAbsolute(relative));
}

function repoRelative(file) {
  const absolute = path.resolve(file);
  assert(inside(ROOT, absolute), "Artifact must remain inside the repository: " + absolute);
  return slash(path.relative(ROOT, absolute));
}

function escapeRegex(value) {
  return String(value).replace(/[.*+?^$(){}|[\]\\]/g, "\\$&");
}

async function exists(file) {
  return fs.stat(file).then(() => true, () => false);
}

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

async function writeJsonNew(file, payload) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  await fs.writeFile(
    file,
    JSON.stringify(payload, null, 2) + "\n",
    { encoding: "utf8", flag: "wx" },
  );
}
function observeRendererErrors(page) {
  const errors = [];
  const onPageError = (error) => errors.push({ type: "pageerror", message: error instanceof Error ? error.message : String(error) });
  const onConsole = (message) => {
    if (message.type() === "error") errors.push({ type: "console", message: message.text() });
  };
  page.on("pageerror", onPageError);
  page.on("console", onConsole);
  return {
    errors,
    settle: () => page.waitForTimeout(RENDERER_ERROR_SETTLE_MS),
    stop: () => {
      page.off("pageerror", onPageError);
      page.off("console", onConsole);
    },
  };
}

async function fileSha256(file) {
  const digest = crypto.createHash("sha256");
  for await (const chunk of createReadStream(file)) digest.update(chunk);
  return digest.digest("hex");
}

async function fileArtifact(file, kind) {
  const stat = await fs.stat(file);
  assert(stat.isFile() && stat.size > 0, kind + " is missing or empty: " + file);
  return {
    kind,
    path: repoRelative(file),
    sha256: await fileSha256(file),
    size_bytes: stat.size,
  };
}

function parseInteger(value, label, minimum, maximum) {
  const parsed = Number(value);
  assert(
    Number.isSafeInteger(parsed) && parsed >= minimum && parsed <= maximum,
    label + " must be an integer from " + minimum + " through " + maximum + ".",
  );
  return parsed;
}

function parseArgs(argv) {
  const allowed = new Set([
    "phase",
    "endpoint",
    "evidence-dir",
    "inventory",
    "timeout-ms",
    "bootstrap-samples",
    "seed",
    "workers",
  ]);
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    assert(token.startsWith("--"), "Unexpected positional argument: " + token);
    const key = token.slice(2);
    assert(allowed.has(key), "Unknown argument: --" + key);
    assert(values[key] === undefined, "Duplicate argument: --" + key);
    const value = argv[index + 1];
    assert(value && !value.startsWith("--"), "Missing value for --" + key);
    values[key] = value;
    index += 1;
  }
  assert(values.phase === "execute" || values.phase === "reopen", "--phase must be execute or reopen.");
  assert(values.endpoint, "--endpoint is required.");
  assert(values["evidence-dir"], "--evidence-dir is required.");
  return {
    ...values,
    timeout: parseInteger(values["timeout-ms"] ?? "300000", "--timeout-ms", 5_000, 900_000),
    bootstrapSamples: parseInteger(
      values["bootstrap-samples"] ?? "1000",
      "--bootstrap-samples",
      100,
      10_000,
    ),
    seed: parseInteger(values.seed ?? "20260822", "--seed", 0, 4_294_967_295),
    workers: parseInteger(values.workers ?? "2", "--workers", 1, 64),
  };
}

function assertLoopbackEndpoint(endpoint) {
  let parsed;
  try {
    parsed = new URL(endpoint);
  } catch {
    throw new Error("--endpoint is not a valid URL: " + endpoint);
  }
  assert(
    ["http:", "https:", "ws:", "wss:"].includes(parsed.protocol),
    "The CDP endpoint must use HTTP(S) or WS(S).",
  );
  assert(
    ["127.0.0.1", "localhost", "::1", "[::1]"].includes(parsed.hostname),
    "The packaged CDP endpoint must be loopback-only.",
  );
  assert(parsed.port, "The packaged CDP endpoint must declare its isolated port.");
  return parsed;
}

function resolveRepoFile(relative, label) {
  assert(
    typeof relative === "string" && relative.trim(),
    label + " must be a non-empty repository-relative path.",
  );
  assert(!path.isAbsolute(relative), label + " must remain repository-relative: " + relative);
  const absolute = path.resolve(ROOT, relative);
  assert(inside(ROOT, absolute), label + " escapes the repository: " + relative);
  return absolute;
}

function jsonPointer(payload, pointer) {
  assert(typeof pointer === "string" && pointer.startsWith("/"), "Invalid JSON pointer: " + pointer);
  return pointer.slice(1).split("/").reduce((value, segment) => {
    if (value === null || typeof value !== "object") return undefined;
    const key = segment.replace(/~1/g, "/").replace(/~0/g, "~");
    return value[key];
  }, payload);
}

async function loadSourceBootstrap(inventoryAbsolute) {
  assert(await exists(inventoryAbsolute), "Reusable archive inventory is missing: " + inventoryAbsolute);
  const inventory = await readJson(inventoryAbsolute);
  assert(
    inventory?.schema === "quickpls.v255.reusable_archive_inventory.v2",
    "Unexpected reusable inventory schema: " + (inventory?.schema ?? "missing"),
  );
  const rows = inventory.public_methods ?? [];
  assert(rows.length === 18, "Reusable inventory must contain exactly 18 public methods.");
  assert(
    new Set(rows.map((row) => row.public_kind)).size === 18,
    "Reusable inventory public method kinds must be unique.",
  );
  const reusableRows = rows.filter((row) => (
    row.reuse_state === "reusable_verified_prior_release"
      && row.new_scientific_run_required === false
  ));
  const newRunRows = rows.filter((row) => row.new_scientific_run_required === true);
  assert(
    reusableRows.length === 17
      && newRunRows.length === 1
      && newRunRows[0].public_kind === POSTHOC_KIND,
    "The inventory must contain 17 reusable rows and only the posthoc new-run gap.",
  );
  const posthocRows = rows.filter((row) => row.public_kind === POSTHOC_KIND);
  assert(
    posthocRows.length === 1
      && posthocRows[0].new_scientific_run_required === true
      && !posthocRows[0].archive_path
      && !posthocRows[0].result_identity,
    "The inventory must still identify posthoc technical minimum sample size as the sole missing packaged run.",
  );
  const sourceRows = rows.filter((row) => row.public_kind === SOURCE_KIND);
  assert(sourceRows.length === 1, "Reusable inventory must contain exactly one PLS bootstrap source row.");
  const row = sourceRows[0];
  assert(
    row.reuse_state === "reusable_verified_prior_release"
      && row.new_scientific_run_required === false,
    "The selected PLS bootstrap source is not reusable verified prior-release evidence.",
  );
  assert(
    row.result_identity?.type === "schema5_result_run_id"
      && typeof row.result_identity.value === "string"
      && row.result_identity.value.trim(),
    "The PLS bootstrap source has no exact schema-5 run identity.",
  );
  assert(typeof row.archive_path === "string", "The PLS bootstrap source has no archive path.");
  assert(
    typeof row.prior_receipt?.path === "string"
      && typeof row.prior_receipt?.json_pointer === "string"
      && row.prior_receipt.verification_status === "passed",
    "The PLS bootstrap source has no passed direct receipt binding.",
  );

  const sourceArchive = resolveRepoFile(row.archive_path, "PLS bootstrap source archive");
  const sourceReceipt = resolveRepoFile(row.prior_receipt.path, "PLS bootstrap source receipt");
  assert(await exists(sourceArchive), "PLS bootstrap source archive is missing: " + row.archive_path);
  assert(await exists(sourceReceipt), "PLS bootstrap source receipt is missing: " + row.prior_receipt.path);
  const sourceReceiptPayload = await readJson(sourceReceipt);
  const boundIdentity = jsonPointer(sourceReceiptPayload, row.prior_receipt.json_pointer);
  assert(
    boundIdentity === row.result_identity.value,
    "The source receipt pointer does not directly bind the declared PLS bootstrap run identity.",
  );
  return {
    inventory,
    inventoryAbsolute,
    row,
    sourceArchive,
    sourceReceipt,
    sourceReceiptBoundIdentity: boundIdentity,
  };
}

async function connectToIsolatedPackagedPage(endpoint, timeout) {
  const parsedEndpoint = assertLoopbackEndpoint(endpoint);
  const browser = await chromium.connectOverCDP(endpoint, { timeout });
  const deadline = Date.now() + timeout;
  let entries = [];
  while (Date.now() < deadline) {
    entries = await enumerateQuickPlsCdpPages(browser);
    if (entries.some(({ state }) => state.shellVisible && state.tauriRuntime)) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const qualifying = entries.filter(({ state }) => state.shellVisible && state.tauriRuntime);
  assert(
    qualifying.length === 1,
    "Expected exactly one shell-visible packaged QuickPLS Tauri page at the isolated endpoint; found "
      + qualifying.length + ": " + JSON.stringify(entries.map(({ state }) => state)),
  );
  assert(
    qualifying[0].state.origin === PACKAGED_TAURI_ORIGIN,
    "Expected packaged origin " + PACKAGED_TAURI_ORIGIN
      + "; received " + (qualifying[0].state.origin ?? "invalid") + ".",
  );
  assert(
    qualifying[0].candidate.viewportSize() === null,
    "The packaged journey must use the actual Tauri client area, not an emulated Playwright viewport.",
  );
  return {
    browser,
    page: qualifying[0].candidate,
    endpointOrigin: parsedEndpoint.origin,
    pageState: qualifying[0].state,
    enumeratedPages: entries.map(({ state }) => state),
  };
}

async function waitForOpenedProjectPath(page, archive, timeout) {
  await page.waitForFunction((target) => {
    const normalize = (value) => String(value ?? "")
      .replace(/\//g, "\\")
      .replace(/\\+/g, "\\")
      .toLowerCase();
    const displayed = document.querySelector(".nd-document-context span")?.textContent ?? "";
    return normalize(displayed) === normalize(target);
  }, archive, { timeout });
}

async function openArchive(page, archive, timeout) {
  await page.evaluate((target) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", {
      detail: { path: target },
    }));
  }, archive);
  await page.locator(".nd-app[data-native-desktop-shell='true']")
    .waitFor({ state: "visible", timeout });
  await waitForOpenedProjectPath(page, archive, timeout);
  await page.waitForTimeout(300);
}

async function navigateToResults(page, timeout) {
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", {
      detail: { surface: "results" },
    }));
  });
  await page.locator(".nd-app[data-surface='results']").waitFor({ state: "visible", timeout });
  await page.locator(".nd-results-workspace").waitFor({ state: "visible", timeout });
}

function resultSelect(page) {
  return page.locator(".nd-results-nav .nd-run-select select").first();
}

async function resultOptions(page) {
  const select = resultSelect(page);
  await select.waitFor({ state: "visible" });
  return select.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: String(option.textContent ?? "").replace(/\s+/g, " ").trim(),
  })));
}

async function selectResultRun(page, runId, timeout) {
  const select = resultSelect(page);
  await select.waitFor({ state: "visible", timeout });
  await page.waitForFunction((expected) => Array.from(
    document.querySelectorAll(".nd-results-nav .nd-run-select select option"),
  ).some((option) => option.value === expected), runId, { timeout });
  await select.selectOption(runId);
  await page.waitForFunction((expected) => (
    document.querySelector(".nd-results-nav .nd-run-select select")?.value === expected
  ), runId, { timeout });
  await page.locator(".nd-results-document .nd-document-tab").waitFor({ state: "visible", timeout });
  const selected = await select.locator("option:checked").first().textContent();
  return { id: await select.inputValue(), label: compact(selected) };
}

function resultTree(page) {
  return page.getByRole("tree", { name: "Available result sections", exact: true });
}

async function expandResultGroup(page, title, timeout) {
  const tree = resultTree(page);
  await tree.waitFor({ state: "visible", timeout });
  const matcher = new RegExp("^" + escapeRegex(title) + "$", "i");
  const group = tree.locator("[role='treeitem'][aria-level='1']").filter({ hasText: matcher });
  assert(await group.count() === 1, "Expected exactly one Results group named " + title + ".");
  if (await group.getAttribute("aria-expanded") !== "true") await group.click();
  await page.waitForFunction((label) => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim().toLowerCase();
    return Array.from(document.querySelectorAll(
      ".nd-result-tree [role='treeitem'][aria-level='1']",
    )).some((node) => clean(node.textContent) === clean(label)
      && node.getAttribute("aria-expanded") === "true");
  }, title, { timeout });
  return group;
}

async function observedTreeLabels(page) {
  const tree = resultTree(page);
  const groups = (await tree.locator("[role='treeitem'][aria-level='1']").allTextContents())
    .map(compact)
    .filter(Boolean);
  const items = (await tree.locator("[role='treeitem'][aria-level='2']").allTextContents())
    .map(compact)
    .filter(Boolean);
  return {
    groups: [...new Set(groups)],
    visible_items: [...new Set(items)],
  };
}

async function chooseResultItem(page, groupTitle, itemTitle, timeout) {
  await expandResultGroup(page, groupTitle, timeout);
  const matcher = new RegExp("^" + escapeRegex(itemTitle) + "$", "i");
  const item = resultTree(page)
    .locator("[role='treeitem'][aria-level='2']")
    .filter({ hasText: matcher });
  assert(
    await item.count() === 1,
    "Expected exactly one Results item named " + itemTitle + " under " + groupTitle + ".",
  );
  await item.click();
  return item;
}

async function verifySourceBootstrapResult(page, sourceRunId, timeout) {
  await navigateToResults(page, timeout);
  const selected = await selectResultRun(page, sourceRunId, timeout);
  assert(selected.id === sourceRunId, "The copied source archive selected the wrong bootstrap run.");
  assert(
    /PLS-SEM Bootstrapping/i.test(selected.label),
    "The declared source run is not presented as PLS-SEM Bootstrapping: " + selected.label,
  );
  await expandResultGroup(page, "Inference", timeout);
  const labels = await observedTreeLabels(page);
  assert(
    labels.groups.some((label) => label === "Inference"),
    "The source bootstrap result has no Inference category.",
  );
  const bootstrapItems = labels.visible_items.filter((label) => /bootstrap/i.test(label));
  assert(
    bootstrapItems.length > 0,
    "The declared source result has no researcher-visible bootstrap evidence.",
  );
  const preferred = [
    "PLS bootstrap replicate accounting",
    "Bootstrapping",
    "Bias-corrected and accelerated intervals",
  ].find((candidate) => labels.visible_items.includes(candidate));
  assert(preferred, "The source result lacks a recognized bootstrap result table.");
  await chooseResultItem(page, "Inference", preferred, timeout);
  const table = page.locator(".nd-result-table-view[data-result-table-id]").first();
  await table.waitFor({ state: "visible", timeout });
  return {
    declared_run_id: sourceRunId,
    selected_run_id: selected.id,
    selected_run_label: selected.label,
    result_categories: labels.groups,
    bootstrap_result_labels: bootstrapItems,
    inspected_bootstrap_item: preferred,
    inspected_table_id: await table.getAttribute("data-result-table-id"),
    passed: true,
  };
}

async function openCalculate(page, timeout) {
  await page.keyboard.press("Control+R");
  const dialog = page.getByRole("dialog", { name: "Calculate", exact: true });
  await dialog.waitFor({ state: "visible", timeout });
  return dialog;
}

async function fillRequiredControl(dialog, selector, value, label) {
  const input = dialog.locator(selector);
  assert(await input.count() === 1, "Expected one " + label + " control (" + selector + ").");
  await input.fill(String(value));
  assert(
    await input.inputValue() === String(value),
    label + " did not retain the requested value " + value + ".",
  );
  return input.inputValue();
}

async function selectRequiredControl(dialog, selector, value, label) {
  const select = dialog.locator(selector);
  assert(await select.count() === 1, "Expected one " + label + " control (" + selector + ").");
  await select.selectOption(value);
  assert(
    await select.inputValue() === value,
    label + " did not retain the requested value " + value + ".",
  );
  return select.inputValue();
}

async function configurePosthocCalculation(page, args) {
  const dialog = await openCalculate(page, args.timeout);
  const options = dialog.locator("#nd-calculation-method-list [role='option']");
  await page.waitForFunction(
    () => document.querySelectorAll("#nd-calculation-method-list [role='option']").length === 18,
    null,
    { timeout: args.timeout },
  );
  const methodCount = await options.count();
  assert(methodCount === 18, "Calculate exposes " + methodCount + " methods instead of exactly 18.");
  const methodLabels = (await options.locator("strong").allTextContents()).map(compact);
  assert(new Set(methodLabels).size === 18, "Calculate method labels are not unique.");

  const option = dialog.locator("#nd-calculation-method-" + POSTHOC_KIND);
  assert(await option.count() === 1, "The posthoc public method is missing from Calculate.");
  await option.click();
  await page.waitForFunction((id) => (
    document.getElementById(id)?.getAttribute("aria-selected") === "true"
  ), "nd-calculation-method-" + POSTHOC_KIND, { timeout: args.timeout });

  const settings = {
    weighting_scheme: await selectRequiredControl(
      dialog,
      "#nd-calculation-weighting",
      "path",
      "weighting scheme",
    ),
    result_data: await selectRequiredControl(
      dialog,
      "#nd-calculation-preprocessing",
      "standardized",
      "result data",
    ),
    maximum_iterations: Number(await fillRequiredControl(
      dialog,
      "#nd-calculation-max-iterations",
      3000,
      "maximum iterations",
    )),
    tolerance: Number(await fillRequiredControl(
      dialog,
      "#nd-calculation-tolerance",
      "0.0000001",
      "stop criterion",
    )),
    bootstrap_samples: Number(await fillRequiredControl(
      dialog,
      "#nd-calculation-bootstrap-samples",
      args.bootstrapSamples,
      "bootstrap samples",
    )),
    confidence_level_percent: Number(await fillRequiredControl(
      dialog,
      "#nd-calculation-confidence",
      95,
      "confidence level",
    )),
    seed: Number(await fillRequiredControl(
      dialog,
      "#nd-calculation-seed",
      args.seed,
      "seed",
    )),
    workers: Number(await fillRequiredControl(
      dialog,
      "#nd-calculation-workers",
      args.workers,
      "parallel workers",
    )),
  };
  const blockers = (await dialog.locator(".nd-blocker li").allTextContents())
    .map(compact)
    .filter(Boolean);
  const start = dialog.getByRole("button", { name: "Start calculation", exact: true });
  assert(await start.count() === 1, "The posthoc calculation has no exact Start action.");
  const selectedMethod = compact(await option.locator("strong").textContent());
  const setup = {
    public_method_kind: POSTHOC_KIND,
    selected_method: selectedMethod,
    selected_aria: await option.getAttribute("aria-selected"),
    public_method_count: methodCount,
    public_method_labels: methodLabels,
    settings,
    studentized_control_count: await dialog.locator("#nd-calculation-studentized").count(),
    blockers,
    start_label: compact(await start.textContent()),
    start_enabled: await start.isEnabled(),
  };
  assert(
    selectedMethod === "Post-hoc Technical Minimum Sample Size"
      && setup.selected_aria === "true",
    "Calculate did not retain the exact posthoc method selection.",
  );
  assert(
    setup.studentized_control_count === 0,
    "The contracted posthoc normal-reference plan unexpectedly exposed studentized inference.",
  );
  assert(blockers.length === 0, "Posthoc setup is blocked: " + JSON.stringify(blockers));
  assert(setup.start_enabled, "Posthoc Start action is disabled without a visible blocker.");
  return { dialog, start, setup };
}

async function captureScreenshot(page, screensRoot, fileName, kind) {
  const file = path.join(screensRoot, fileName);
  assert(!await exists(file), "Refusing to overwrite screenshot: " + file);
  await fs.mkdir(path.dirname(file), { recursive: true });
  await page.screenshot({ path: file, animations: "disabled" });
  return fileArtifact(file, kind);
}

async function captureProgressIfPerceptible(page, screensRoot) {
  const progress = page.locator(
    ".nd-dialog-calculation .nd-run-progress[aria-busy='true']:is(.queued,.validating,.running)",
  );
  try {
    await progress.waitFor({ state: "visible", timeout: 5_000 });
  } catch (error) {
    return {
      perceptible: false,
      screenshot: null,
      detail: "Calculation completed before a stable progress state could be captured: "
        + (error instanceof Error ? error.message : String(error)),
    };
  }
  const state = await progress.evaluate((node) => ({
    status: ["queued", "validating", "running"].find((name) => node.classList.contains(name)) ?? null,
    aria_busy: node.getAttribute("aria-busy"),
    phase: String(node.querySelector("strong")?.textContent ?? "").replace(/\s+/g, " ").trim(),
    message: String(node.querySelector("p")?.textContent ?? "").replace(/\s+/g, " ").trim(),
    progress_value: node.querySelector("progress")?.getAttribute("value") ?? null,
    progress_max: node.querySelector("progress")?.getAttribute("max") ?? null,
    log_entries: node.querySelectorAll("ol li").length,
  }));
  return {
    perceptible: true,
    state,
    screenshot: await captureScreenshot(
      page,
      screensRoot,
      "02-progress.png",
      "posthoc_progress",
    ),
  };
}

async function startAndWaitForNewPosthocRun(page, start, beforeOptions, timeout, screensRoot) {
  const beforeValues = beforeOptions.map((option) => option.value);
  await start.click();
  const progress = await captureProgressIfPerceptible(page, screensRoot);
  const handle = await page.waitForFunction(({ previousValues }) => {
    const dialog = document.querySelector(".nd-dialog-calculation[role='dialog']");
    const terminal = dialog?.querySelector(
      ".nd-run-progress.failed, .nd-run-progress.cancelled",
    );
    if (terminal) {
      return {
        state: terminal.classList.contains("failed") ? "failed" : "cancelled",
        detail: String(terminal.textContent ?? "").replace(/\s+/g, " ").trim(),
      };
    }
    const options = Array.from(
      document.querySelectorAll(".nd-results-nav .nd-run-select select option"),
    ).map((option) => ({
      value: option.value,
      label: String(option.textContent ?? "").replace(/\s+/g, " ").trim(),
    }));
    const added = options.filter((option) => option.value && !previousValues.includes(option.value));
    if (!dialog && added.length > 0) return { state: "completed", added };
    return null;
  }, { previousValues: beforeValues }, { timeout });
  const terminal = await handle.jsonValue();
  await handle.dispose();
  assert(
    terminal?.state === "completed",
    "Posthoc calculation ended in " + (terminal?.state ?? "an unknown state")
      + ": " + (terminal?.detail ?? "no terminal detail"),
  );
  assert(
    terminal.added.length === 1,
    "The posthoc calculation added " + terminal.added.length
      + " Results entries instead of exactly one: " + JSON.stringify(terminal.added),
  );
  const posthoc = terminal.added.filter((option) => EXPECTED_POSTHOC_RUN_LABEL.test(option.label));
  assert(
    posthoc.length === 1 && posthoc[0].value,
    "The newly added Results entry is not the posthoc technical minimum sample-size run: "
      + JSON.stringify(terminal.added),
  );
  return { progress, newResult: posthoc[0], addedResults: terminal.added };
}

async function posthocResultEvidence(page, runId, timeout) {
  await navigateToResults(page, timeout);
  const selected = await selectResultRun(page, runId, timeout);
  assert(selected.id === runId, "Results did not retain the new posthoc run identity.");
  assert(
    EXPECTED_POSTHOC_RUN_LABEL.test(selected.label),
    "The selected new run has an unexpected label: " + selected.label,
  );
  await chooseResultItem(page, "Inference", EXPECTED_POSTHOC_RESULT_TITLE, timeout);
  const table = page.locator(
    ".nd-result-table-view[data-result-table-id='posthoc_minimum_sample_size']",
  );
  await table.waitFor({ state: "visible", timeout });
  const tableTitle = compact(await table.locator("h1").textContent());
  assert(
    tableTitle === EXPECTED_POSTHOC_RESULT_TITLE,
    "The posthoc result table has an unexpected researcher-facing title: " + tableTitle,
  );
  const rows = await table.locator("tbody tr").evaluateAll((nodes) => nodes.map((node) => {
    const cells = Array.from(node.querySelectorAll("th,td"))
      .map((cell) => String(cell.textContent ?? "").replace(/\s+/g, " ").trim());
    return { label: cells[0] ?? "", value: cells[1] ?? "" };
  }));
  const byLabel = Object.fromEntries(rows.map((row) => [row.label, row.value]));
  const requiredLabels = [
    "Technically required sample size",
    "Analytical sample size",
    "Technical requirement",
    "Driving path",
    "Absolute path coefficient",
    "Bootstrap p value (two-sided)",
    "Significant structural paths",
    "Driver selection",
    "Formula assumptions",
    "Method",
  ];
  for (const label of requiredLabels) {
    assert(
      typeof byLabel[label] === "string" && byLabel[label].trim(),
      "The completed posthoc table is missing the required row " + label + ".",
    );
  }
  assert(
    byLabel["Result status"] === undefined,
    "The posthoc run completed only with an unavailable/incomplete Result status.",
  );
  assert(
    /^\d+$/.test(byLabel["Technically required sample size"])
      && Number(byLabel["Technically required sample size"]) > 0,
    "The posthoc technical minimum sample size is not a positive integer.",
  );
  assert(
    /^\d+$/.test(byLabel["Analytical sample size"])
      && Number(byLabel["Analytical sample size"]) > 0,
    "The posthoc analytical sample size is not a positive integer.",
  );
  assert(
    /^(Met|Not met)$/.test(byLabel["Technical requirement"]),
    "The posthoc technical requirement has no determinate state.",
  );
  assert(
    /→|->/.test(byLabel["Driving path"]),
    "The posthoc driving path is not researcher-readable.",
  );
  assert(
    Number.isFinite(Number(byLabel["Absolute path coefficient"])),
    "The posthoc absolute path coefficient is not numeric.",
  );
  assert(
    Number.isFinite(Number(byLabel["Bootstrap p value (two-sided)"])),
    "The posthoc linked two-sided bootstrap probability is not numeric.",
  );
  assert(
    /^\d+$/.test(byLabel["Significant structural paths"])
      && Number(byLabel["Significant structural paths"]) > 0,
    "The posthoc run did not identify any statistically significant structural path.",
  );
  assert(
    byLabel["Driver selection"]
      === "Smallest absolute path with two-sided normal-reference bootstrap p ≤ 0.05",
    "The completed posthoc result did not retain the significance-aware driver selection rule.",
  );
  assert(
    byLabel.Method === "Inverse square root",
    "The posthoc result did not identify the inverse-square-root method.",
  );
  const labels = await observedTreeLabels(page);
  assert(
    labels.groups.includes("Inference")
      && labels.visible_items.includes(EXPECTED_POSTHOC_RESULT_TITLE),
    "The completed posthoc result is not researcher-accessible under Inference.",
  );
  return {
    selected_run_id: selected.id,
    selected_run_label: selected.label,
    observed_result_categories: labels.groups,
    observed_result_labels: labels.visible_items,
    selected_table_id: await table.getAttribute("data-result-table-id"),
    selected_table_title: tableTitle,
    rows,
    row_values: byLabel,
    bootstrap_link_verified: true,
  };
}

async function saveCurrentArchive(page, archive, initialSha256, screensRoot, timeout) {
  await page.keyboard.press("Control+S");
  const toast = page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last();
  await toast.waitFor({ state: "visible", timeout });
  const toastText = compact(await toast.textContent());
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout });
  const screenshot = await captureScreenshot(
    page,
    screensRoot,
    "04-saved.png",
    "posthoc_saved_results",
  );
  await page.waitForTimeout(400);
  const finalArtifact = await fileArtifact(archive, "saved_posthoc_archive");
  assert(
    finalArtifact.sha256 !== initialSha256,
    "Control+S did not change the copied archive after the new posthoc result completed.",
  );
  return {
    toast: toastText,
    title_clean: true,
    archive: finalArtifact,
    changed_from_copied_source: true,
    screenshot,
  };
}

function processSafety(connection) {
  return {
    attached_endpoint_origin: connection.endpointOrigin,
    packaged_page: connection.pageState,
    enumerated_pages: connection.enumeratedPages,
    wrapper_owns_exact_pid_lifecycle: true,
    driver_launches_app_processes: false,
    driver_terminates_app_processes: false,
    driver_closes_browser_page_or_context: false,
  };
}

async function executeJourney(args, evidenceDir, inventoryAbsolute) {
  await fs.mkdir(evidenceDir, { recursive: true });
  const screensRoot = path.join(evidenceDir, "screens");
  const targetArchive = path.join(evidenceDir, TARGET_ARCHIVE_NAME);
  const executeReceipt = path.join(evidenceDir, EXECUTE_RECEIPT_NAME);
  const reopenReceipt = path.join(evidenceDir, REOPEN_RECEIPT_NAME);
  for (const target of [targetArchive, executeReceipt, reopenReceipt]) {
    assert(!await exists(target), "Refusing to reuse or overwrite execute evidence: " + target);
  }
  assert(
    !await exists(screensRoot),
    "Refusing to reuse or overlap an existing posthoc screenshot directory: " + screensRoot,
  );

  const source = await loadSourceBootstrap(inventoryAbsolute);
  const sourceArchiveArtifact = await fileArtifact(source.sourceArchive, "source_pls_bootstrap_archive");
  const sourceReceiptArtifact = await fileArtifact(source.sourceReceipt, "source_pls_bootstrap_receipt");
  const inventoryArtifact = await fileArtifact(source.inventoryAbsolute, "reusable_archive_inventory");
  await fs.copyFile(source.sourceArchive, targetArchive, fsConstants.COPYFILE_EXCL);
  const copiedArtifact = await fileArtifact(targetArchive, "copied_working_archive_before_posthoc");
  assert(
    copiedArtifact.sha256 === sourceArchiveArtifact.sha256,
    "The copied working archive does not match the verified PLS bootstrap source archive.",
  );

  const connection = await connectToIsolatedPackagedPage(args.endpoint, args.timeout);
  const page = connection.page;
  const rendererErrors = observeRendererErrors(page);
  await openArchive(page, targetArchive, args.timeout);
  const sourceBootstrapVerification = await verifySourceBootstrapResult(
    page,
    source.row.result_identity.value,
    args.timeout,
  );
  const beforeOptions = await resultOptions(page);
  assert(
    beforeOptions.some((option) => option.value === source.row.result_identity.value),
    "The copied archive lost the declared PLS bootstrap source result before calculation.",
  );

  const configured = await configurePosthocCalculation(page, args);
  const setupScreenshot = await captureScreenshot(
    page,
    screensRoot,
    "01-setup.png",
    "posthoc_calculate_setup",
  );
  const lifecycle = await startAndWaitForNewPosthocRun(
    page,
    configured.start,
    beforeOptions,
    args.timeout,
    screensRoot,
  );
  const resultEvidence = await posthocResultEvidence(
    page,
    lifecycle.newResult.value,
    args.timeout,
  );
  const resultScreenshot = await captureScreenshot(
    page,
    screensRoot,
    "03-results.png",
    "posthoc_completed_results",
  );
  const save = await saveCurrentArchive(
    page,
    targetArchive,
    copiedArtifact.sha256,
    screensRoot,
    args.timeout,
  );
  const screenshots = [
    setupScreenshot,
    ...(lifecycle.progress.screenshot ? [lifecycle.progress.screenshot] : []),
    resultScreenshot,
    save.screenshot,
  ];
  await rendererErrors.settle();
  const consoleErrors = [...rendererErrors.errors];
  rendererErrors.stop();
  const executeReceiptRelative = repoRelative(executeReceipt);
  const archiveRelative = repoRelative(targetArchive);
  const receipt = {
    schema_version: 1,
    schema: "quickpls.v255.posthoc_minimum_sample_packaged_smoke.v1",
    suite_id: "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1",
    target_release: TARGET_RELEASE,
    version_authority: "2.55.0 candidate after the consolidated source gate",
    generated_at: new Date().toISOString(),
    status: consoleErrors.length === 0 ? "passed" : "failed",
    phase: "execute",
    console_errors: consoleErrors,
    named_evidence_observations: [],
    public_kind: POSTHOC_KIND,
    method_kind: POSTHOC_KIND,
    reuse_state: "new_scientific_run_supplied",
    new_scientific_run_required: false,
    current_ui_capture_required: false,
    source_release: "2.55.0-candidate",
    source_bootstrap_result_id: source.row.result_identity.value,
    new_result_id: lifecycle.newResult.value,
    result_identity: {
      type: "schema5_result_run_id",
      value: lifecycle.newResult.value,
    },
    scientific_identity: {
      capability_cell: "qpls3.pls.posthoc_technical_minimum_sample_size",
      capability_version: "pls_posthoc_technical_minimum_sample_size_v2",
      method_version: "inverse_square_root_posthoc_v2",
      selection_rule: "smallest_absolute_statistically_significant_structural_path",
      significance_source: "pls_bootstrap_normal_reference_two_sided",
    },
    archive_path: archiveRelative,
    archive_sha256: save.archive.sha256,
    archive: {
      ...save.archive,
      copied_from: sourceArchiveArtifact.path,
      source_sha256: sourceArchiveArtifact.sha256,
      copied_before_run_sha256: copiedArtifact.sha256,
      changed_after_save: save.changed_from_copied_source,
    },
    prior_receipt: {
      path: executeReceiptRelative,
      json_pointer: "/new_result_id",
      verification_status: "passed",
    },
    source_bootstrap: {
      inventory: inventoryArtifact,
      inventory_row_public_kind: source.row.public_kind,
      scientific_identity: source.row.scientific_identity ?? null,
      source_release: source.row.source_release ?? null,
      archive: sourceArchiveArtifact,
      receipt: {
        ...sourceReceiptArtifact,
        json_pointer: source.row.prior_receipt.json_pointer,
        pointer_value: source.sourceReceiptBoundIdentity,
        declared_identity_directly_bound: true,
      },
      prior_screenshots: source.row.prior_screenshots ?? [],
      verification: sourceBootstrapVerification,
    },
    calculate_setup: configured.setup,
    source_bootstrap_link: {
      same_loaded_project_archive: true,
      declared_source_bootstrap_run_present: true,
      contracted_posthoc_bootstrap_settings_verified: true,
      new_run_uses_linked_full_model_bootstrap_plan: true,
      prior_source_run_numerically_reused_by_engine: false,
      note: "The prior run proves a qualifying completed bootstrap archive and model. The posthoc method executes its own linked case-bootstrap plan; it does not claim to reuse prior replicate values.",
    },
    lifecycle: {
      progress: lifecycle.progress,
      added_results: lifecycle.addedResults,
      terminal_state: "completed",
      save: {
        toast: save.toast,
        title_clean: save.title_clean,
      },
    },
    observed_result_categories: resultEvidence.observed_result_categories,
    observed_result_labels: resultEvidence.observed_result_labels,
    observed_posthoc_rows: resultEvidence.rows,
    result_verification: resultEvidence,
    screenshots,
    reopen_verification: {
      status: "pending_fresh_process_phase",
      receipt_path: repoRelative(reopenReceipt),
    },
    process_safety: processSafety(connection),
  };
  await writeJsonNew(executeReceipt, receipt);
  assert(receipt.status === "passed", `Renderer errors were observed: ${JSON.stringify(receipt.console_errors)}`);
  return {
    passed: true,
    phase: "execute",
    method_kind: POSTHOC_KIND,
    source_bootstrap_result_id: receipt.source_bootstrap_result_id,
    new_result_id: receipt.new_result_id,
    archive: receipt.archive_path,
    receipt: executeReceiptRelative,
    reopen_required: true,
  };
}

async function reopenJourney(args, evidenceDir) {
  const executeReceipt = path.join(evidenceDir, EXECUTE_RECEIPT_NAME);
  const reopenReceipt = path.join(evidenceDir, REOPEN_RECEIPT_NAME);
  assert(await exists(executeReceipt), "Execute receipt is required before reopen: " + executeReceipt);
  assert(!await exists(reopenReceipt), "Refusing to overwrite reopen receipt: " + reopenReceipt);
  const executed = await readJson(executeReceipt);
  assert(
    executed?.schema === "quickpls.v255.posthoc_minimum_sample_packaged_smoke.v1"
      && executed.status === "passed"
      && executed.phase === "execute"
      && Array.isArray(executed.console_errors)
      && executed.console_errors.length === 0
      && executed.public_kind === POSTHOC_KIND,
    "The execute receipt is not a passed, renderer-clean posthoc packaged journey.",
  );
  assert(
    executed.result_identity?.type === "schema5_result_run_id"
      && executed.result_identity.value === executed.new_result_id
      && executed.new_result_id,
    "The execute receipt does not bind one exact new posthoc run identity.",
  );
  assert(
    typeof executed.source_bootstrap_result_id === "string"
      && executed.source_bootstrap_result_id.trim(),
    "The execute receipt lost its source bootstrap run identity.",
  );
  const archive = resolveRepoFile(executed.archive_path, "saved posthoc archive");
  assert(
    inside(evidenceDir, archive),
    "The saved posthoc archive is outside the selected evidence directory.",
  );
  assert(await exists(archive), "The saved posthoc archive is missing: " + archive);
  const archiveArtifact = await fileArtifact(archive, "reopened_posthoc_archive");
  assert(
    archiveArtifact.sha256 === executed.archive_sha256,
    "The saved posthoc archive changed after execute evidence was recorded.",
  );

  const connection = await connectToIsolatedPackagedPage(args.endpoint, args.timeout);
  const page = connection.page;
  const rendererErrors = observeRendererErrors(page);
  await openArchive(page, archive, args.timeout);
  const resultEvidence = await posthocResultEvidence(
    page,
    executed.new_result_id,
    args.timeout,
  );
  assert(
    resultEvidence.selected_run_id === executed.new_result_id,
    "Fresh reopen changed the posthoc result identity.",
  );
  const screenshot = await captureScreenshot(
    page,
    path.join(evidenceDir, "screens"),
    "05-reopen.png",
    "posthoc_fresh_reopen",
  );
  await rendererErrors.settle();
  const consoleErrors = [...rendererErrors.errors];
  rendererErrors.stop();
  const executeReceiptArtifact = await fileArtifact(executeReceipt, "posthoc_execute_receipt");
  const receipt = {
    schema_version: 1,
    schema: "quickpls.v255.posthoc_minimum_sample_reopen.v1",
    suite_id: "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1",
    target_release: TARGET_RELEASE,
    generated_at: new Date().toISOString(),
    status: consoleErrors.length === 0 ? "passed" : "failed",
    phase: "reopen",
    console_errors: consoleErrors,
    named_evidence_observations: [],
    public_kind: POSTHOC_KIND,
    method_kind: POSTHOC_KIND,
    source_bootstrap_result_id: executed.source_bootstrap_result_id,
    new_result_id: executed.new_result_id,
    result_identity: executed.result_identity,
    archive_path: executed.archive_path,
    archive_sha256: archiveArtifact.sha256,
    archive: archiveArtifact,
    execute_receipt: executeReceiptArtifact,
    same_result_identity: true,
    same_archive_sha256: true,
    observed_result_categories: resultEvidence.observed_result_categories,
    observed_result_labels: resultEvidence.observed_result_labels,
    observed_posthoc_rows: resultEvidence.rows,
    result_verification: resultEvidence,
    screenshots: [screenshot],
    process_safety: processSafety(connection),
  };
  await writeJsonNew(reopenReceipt, receipt);
  assert(receipt.status === "passed", `Renderer errors were observed: ${JSON.stringify(receipt.console_errors)}`);
  return {
    passed: true,
    phase: "reopen",
    method_kind: POSTHOC_KIND,
    source_bootstrap_result_id: receipt.source_bootstrap_result_id,
    new_result_id: receipt.new_result_id,
    same_result_identity: true,
    archive: receipt.archive_path,
    receipt: repoRelative(reopenReceipt),
  };
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  const evidenceDir = path.resolve(args["evidence-dir"]);
  assert(
    inside(RESULTS_ROOT, evidenceDir) && evidenceDir !== RESULTS_ROOT,
    "--evidence-dir must be a dedicated child directory below validation/results.",
  );
  const inventoryAbsolute = args.inventory
    ? resolveRepoFile(args.inventory, "--inventory")
    : DEFAULT_INVENTORY;
  if (args.phase === "execute") {
    return executeJourney(args, evidenceDir, inventoryAbsolute);
  }
  assert(await exists(evidenceDir), "Reopen evidence directory does not exist: " + evidenceDir);
  return reopenJourney(args, evidenceDir);
}

let outcome;
let exitCode = 0;
try {
  outcome = await run();
} catch (error) {
  exitCode = 1;
  outcome = {
    passed: false,
    method_kind: POSTHOC_KIND,
    failure: error instanceof Error ? error.message : String(error),
    note: "The driver did not launch, terminate, or close the packaged QuickPLS process.",
  };
}

// Never call browser.close() here. For a local connectOverCDP Browser object it
// may send Browser.close, violating the wrapper-owned exact-PID safety boundary.
// All filesystem and screenshot writes are awaited before this driver process
// exits and merely drops its own CDP transport sockets.
process.stdout.write(JSON.stringify(outcome, null, 2) + "\n", () => {
  process.exit(exitCode);
});
