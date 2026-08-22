#!/usr/bin/env node
/**
 * QuickPLS 2.55 frozen-archive Results capture driver.
 *
 * This source-only driver attaches to one already-running, isolated packaged
 * Tauri WebView2 CDP endpoint. Process creation, exact-PID ownership checks,
 * and termination belong exclusively to the wrapper. This file deliberately
 * does not import child_process and never closes a page, browser, or context.
 * The driver exits its own Node process after all writes have been awaited,
 * which disconnects CDP without sending Browser.close to QuickPLS.
 *
 * Required arguments:
 *   --endpoint http://127.0.0.1:<isolated-port>
 *   --staging-dir validation/results/<new-empty-directory>
 *
 * Optional arguments:
 *   --inventory validation/v255_reusable_archive_inventory.json
 *   --posthoc-supplement validation/results/<new-run-descriptor>.json
 *   --timeout-ms 60000
 *
 * A posthoc supplement has the same archive/result_identity/prior_receipt
 * fields as an inventory row and must bind its new result identity directly
 * through the declared receipt JSON pointer. Without it, the other 17 rows are
 * captured and the crawl finishes non-zero with an explicit posthoc receipt.
 */

import crypto from "node:crypto";
import { createReadStream } from "node:fs";
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
const TARGET_RELEASE = "2.55.0";
const RENDERER_ERROR_SETTLE_MS = 250;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function slash(value) {
  return String(value).split(path.sep).join("/");
}

function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

async function exists(file) {
  return fs.stat(file).then(() => true, () => false);
}

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

async function writeJsonNew(file, payload) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  await fs.writeFile(file, `${JSON.stringify(payload, null, 2)}\n`, { encoding: "utf8", flag: "wx" });
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

async function namedObservationFromReceipt({ staging, receiptArtifact, caseId, observed }) {
  const receiptPath = path.join(staging, ...receiptArtifact.member.split("/"));
  const receipt = await readJson(receiptPath);
  assert(receipt.status === "verified_current_ui_capture", `${caseId} source receipt is not a verified current UI capture.`);
  const screenshotPath = path.join(staging, ...receipt.screenshot.member.split("/"));
  assert(inside(staging, screenshotPath) && await exists(screenshotPath), `${caseId} screenshot is missing from frozen staging.`);
  const screenshotHash = await fileSha256(screenshotPath);
  assert(screenshotHash === receipt.screenshot.sha256, `${caseId} screenshot does not match its frozen receipt.`);
  const operation = "capture_observability_state";
  return {
    schema_version: 1,
    case_id: caseId,
    operation,
    assertion: {
      id: `${operation}:${caseId}`,
      passed: true,
      expected: observed,
      observed,
    },
    screenshot: {
      path: screenshotPath,
      sha256: screenshotHash,
    },
  };
}

function safeFileName(value) {
  const name = String(value).replace(/[^a-z0-9._-]+/giu, "-").replace(/^-+|-+$/g, "");
  return name || "artifact";
}

function parseArgs(argv) {
  const allowed = new Set(["endpoint", "staging-dir", "inventory", "posthoc-supplement", "timeout-ms"]);
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    assert(token.startsWith("--"), `Unexpected positional argument: ${token}`);
    const key = token.slice(2);
    assert(allowed.has(key), `Unknown argument: --${key}`);
    const value = argv[index + 1];
    assert(value && !value.startsWith("--"), `Missing value for --${key}`);
    values[key] = value;
    index += 1;
  }
  assert(values.endpoint, "--endpoint is required.");
  assert(values["staging-dir"], "--staging-dir is required.");
  const timeout = Number(values["timeout-ms"] ?? 60_000);
  assert(Number.isInteger(timeout) && timeout >= 5_000 && timeout <= 300_000, "--timeout-ms must be an integer from 5000 through 300000.");
  return { ...values, timeout };
}

function assertLoopbackEndpoint(endpoint) {
  let parsed;
  try {
    parsed = new URL(endpoint);
  } catch {
    throw new Error(`--endpoint is not a valid URL: ${endpoint}`);
  }
  assert(["http:", "https:", "ws:", "wss:"].includes(parsed.protocol), "The CDP endpoint must use HTTP(S) or WS(S).");
  assert(["127.0.0.1", "localhost", "::1", "[::1]"].includes(parsed.hostname), "The packaged CDP endpoint must be loopback-only.");
  assert(parsed.port, "The packaged CDP endpoint must declare its isolated port.");
}

function resolveRepoFile(relative, label) {
  assert(typeof relative === "string" && relative.trim(), `${label} must be a non-empty repository-relative path.`);
  assert(!path.isAbsolute(relative), `${label} must remain repository-relative: ${relative}`);
  const absolute = path.resolve(ROOT, relative);
  assert(inside(ROOT, absolute), `${label} escapes the repository: ${relative}`);
  return absolute;
}

function jsonPointer(payload, pointer) {
  assert(typeof pointer === "string" && pointer.startsWith("/"), `Invalid JSON pointer: ${pointer}`);
  return pointer.slice(1).split("/").reduce((value, segment) => {
    if (value === null || typeof value !== "object") return undefined;
    const key = segment.replace(/~1/g, "/").replace(/~0/g, "~");
    return value[key];
  }, payload);
}

function pointerValueSummary(value) {
  if (value === null || ["string", "number", "boolean"].includes(typeof value)) return value;
  if (Array.isArray(value)) return { type: "array", length: value.length };
  if (typeof value === "object") return { type: "object", keys: Object.keys(value).sort() };
  return { type: typeof value };
}

function validateResultIdentity(identity, kind) {
  assert(identity && typeof identity === "object", `${kind} has no declared result identity.`);
  assert(["canonical_result_document_id", "schema5_result_run_id"].includes(identity.type), `${kind} has unsupported identity type '${identity.type}'.`);
  assert(typeof identity.value === "string" && identity.value.trim(), `${kind} has an empty declared identity.`);
  if (identity.type === "canonical_result_document_id") {
    assert(identity.value.startsWith("result_"), `${kind} canonical result-document ID must start with 'result_'.`);
  }
}

function validateInventory(inventory) {
  assert(inventory?.schema === "quickpls.v255.reusable_archive_inventory.v1", `Unexpected reusable inventory schema: ${inventory?.schema ?? "missing"}`);
  const rows = inventory.public_methods ?? [];
  const kinds = rows.map((row) => row.public_kind);
  assert(rows.length === 18, `Reusable inventory contains ${rows.length} public methods instead of 18.`);
  assert(new Set(kinds).size === 18, "Reusable inventory public method kinds are not unique.");
  const reusable = rows.filter((row) => row.reuse_state === "reusable_verified_prior_release" && row.new_scientific_run_required === false);
  assert(reusable.length === 17, `Reusable inventory contains ${reusable.length} reusable rows instead of 17.`);
  const pending = rows.filter((row) => row.new_scientific_run_required === true);
  assert(pending.length === 1 && pending[0].public_kind === POSTHOC_KIND, "The sole new-run row must be the posthoc technical minimum sample-size method.");
  for (const row of reusable) {
    validateResultIdentity(row.result_identity, row.public_kind);
    assert(typeof row.archive_path === "string", `${row.public_kind} has no archive path.`);
    assert(typeof row.prior_receipt?.path === "string" && typeof row.prior_receipt?.json_pointer === "string", `${row.public_kind} has no prior receipt binding.`);
  }
  return rows;
}

async function loadPosthocSupplement(file) {
  if (!file) return null;
  const absolute = resolveRepoFile(file, "--posthoc-supplement");
  assert(await exists(absolute), `Posthoc supplement does not exist: ${file}`);
  const supplement = await readJson(absolute);
  assert(supplement?.public_kind === POSTHOC_KIND, `Posthoc supplement public_kind must be '${POSTHOC_KIND}'.`);
  assert(supplement?.schema === "quickpls.v255.posthoc_minimum_sample_packaged_smoke.v1"
    && supplement?.suite_id === "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1"
    && supplement?.target_release === TARGET_RELEASE
    && supplement?.status === "passed"
    && supplement?.phase === "execute"
    && Array.isArray(supplement?.console_errors)
    && supplement.console_errors.length === 0, "Posthoc supplement must be the passed, renderer-clean packaged execute receipt for QuickPLS 2.55.");
  validateResultIdentity(supplement.result_identity, POSTHOC_KIND);
  assert(supplement.new_result_id === supplement.result_identity.value, "Posthoc supplement result identity does not bind its completed run.");
  assert(supplement.scientific_identity?.capability_cell === "qpls3.pls.posthoc_technical_minimum_sample_size"
    && supplement.scientific_identity?.capability_version === "pls_posthoc_technical_minimum_sample_size_v2"
    && supplement.scientific_identity?.method_version === "inverse_square_root_posthoc_v2", "Posthoc supplement does not bind the exact qualified capability and method versions.");
  assert(supplement.lifecycle?.terminal_state === "completed"
    && supplement.result_verification?.selected_run_id === supplement.new_result_id
    && supplement.result_verification?.selected_table_title === "Post-hoc minimum sample size", "Posthoc supplement does not prove one complete researcher-facing Result.");
  assert(typeof supplement.archive_path === "string", "Posthoc supplement must provide archive_path.");
  assert(typeof supplement.prior_receipt?.path === "string" && typeof supplement.prior_receipt?.json_pointer === "string", "Posthoc supplement must provide a receipt path and direct identity JSON pointer.");
  const reopenRelative = supplement.reopen_verification?.receipt_path;
  assert(typeof reopenRelative === "string", "Posthoc supplement must point to its fresh-process reopen receipt.");
  const reopenAbsolute = resolveRepoFile(reopenRelative, "posthoc reopen receipt");
  assert(await exists(reopenAbsolute), `Posthoc fresh-process reopen receipt is missing: ${reopenRelative}`);
  const reopened = await readJson(reopenAbsolute);
  assert(reopened?.schema === "quickpls.v255.posthoc_minimum_sample_reopen.v1"
    && reopened?.suite_id === "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1"
    && reopened?.target_release === TARGET_RELEASE
    && reopened?.status === "passed"
    && reopened?.phase === "reopen"
    && Array.isArray(reopened?.console_errors)
    && reopened.console_errors.length === 0
    && reopened?.same_result_identity === true
    && reopened?.same_archive_sha256 === true
    && reopened?.new_result_id === supplement.new_result_id
    && reopened?.archive_sha256 === supplement.archive_sha256
    && reopened?.result_verification?.selected_run_id === supplement.new_result_id
    && reopened?.result_verification?.selected_table_title === "Post-hoc minimum sample size", "Posthoc supplement fresh-process reopen receipt is incomplete or identity-inconsistent.");
  return {
    ...supplement,
    reuse_state: "new_scientific_run_supplied",
    new_scientific_run_required: false,
    current_ui_capture_required: true,
    reopen_verification: {
      status: "verified_fresh_process",
      receipt_path: reopenRelative,
      receipt_sha256: await fileSha256(reopenAbsolute),
    },
  };
}

async function stageArtifact({ source, sourceRelative, directory, staging, cache }) {
  const sha256 = await fileSha256(source);
  const stat = await fs.stat(source);
  const cacheKey = `${directory}\0${sha256}`;
  const cached = cache.get(cacheKey);
  if (cached) return { ...cached, source_path: sourceRelative };
  const member = slash(path.join(directory, `${sha256.slice(0, 16)}-${safeFileName(path.basename(source))}`));
  const destination = path.join(staging, ...member.split("/"));
  await fs.mkdir(path.dirname(destination), { recursive: true });
  await fs.copyFile(source, destination);
  const artifact = { member, sha256, size_bytes: stat.size };
  cache.set(cacheKey, artifact);
  return { ...artifact, source_path: sourceRelative };
}

async function connectToIsolatedPackagedPage(endpoint, timeout) {
  assertLoopbackEndpoint(endpoint);
  const browser = await chromium.connectOverCDP(endpoint, { timeout });
  const deadline = Date.now() + timeout;
  let entries = [];
  while (Date.now() < deadline) {
    entries = await enumerateQuickPlsCdpPages(browser);
    if (entries.some(({ state }) => state.shellVisible && state.tauriRuntime)) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const qualifying = entries.filter(({ state }) => state.shellVisible && state.tauriRuntime);
  assert(qualifying.length === 1, `Expected exactly one shell-visible packaged QuickPLS Tauri page at the isolated endpoint; found ${qualifying.length}: ${JSON.stringify(entries.map(({ state }) => state))}`);
  assert(qualifying[0].state.origin === PACKAGED_TAURI_ORIGIN, `Expected packaged origin ${PACKAGED_TAURI_ORIGIN}; received ${qualifying[0].state.origin ?? "invalid"}.`);
  assert(qualifying[0].candidate.viewportSize() === null, "The packaged crawler must use the actual Tauri client area, not an emulated Playwright viewport.");
  return {
    browser,
    page: qualifying[0].candidate,
    page_state: qualifying[0].state,
    enumerated_pages: entries.map(({ state }) => state),
  };
}

async function waitForOpenedProjectPath(page, archive, timeout) {
  await page.waitForFunction((target) => {
    const normalize = (value) => String(value ?? "").replace(/\//g, "\\").replace(/\\+/g, "\\").toLowerCase();
    const displayed = document.querySelector(".nd-document-context span")?.textContent ?? "";
    return normalize(displayed) === normalize(target);
  }, archive, { timeout });
}

async function openArchive(page, archive, timeout) {
  await page.evaluate((target) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } }));
  }, archive);
  await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
  await waitForOpenedProjectPath(page, archive, timeout);
  await page.waitForTimeout(250);
}

function expectedResultOption(identity) {
  return identity.type === "canonical_result_document_id"
    ? `canonical:${identity.value}`
    : identity.value;
}

async function navigateToDeclaredResult(page, row, timeout) {
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "results" } }));
  });
  await page.locator('.nd-app[data-surface="results"]').waitFor({ state: "visible", timeout });
  const workspace = page.locator(".nd-results-workspace");
  await workspace.waitFor({ state: "visible", timeout });
  const select = page.locator(".nd-results-nav .nd-run-select select").first();
  await select.waitFor({ state: "visible", timeout });
  const expected = expectedResultOption(row.result_identity);
  await page.waitForFunction((optionValue) => Array.from(
    document.querySelectorAll(".nd-results-nav .nd-run-select select option"),
  ).some((option) => option.value === optionValue), expected, { timeout });
  await select.selectOption(expected);
  await page.waitForFunction((optionValue) => (
    document.querySelector(".nd-results-nav .nd-run-select select")?.value === optionValue
  ), expected, { timeout });
  await page.locator(".nd-results-document .nd-document-tab").waitFor({ state: "visible", timeout });

  const canonicalIdentity = await page.locator(".nd-canonical-export-v2 details dl > div").evaluateAll((nodes) => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
    return Object.fromEntries(nodes.map((node) => [
      clean(node.querySelector("dt")?.textContent),
      clean(node.querySelector("dd")?.textContent),
    ]));
  }).catch(() => ({}));
  const selectedValue = await select.inputValue();
  if (row.result_identity.type === "canonical_result_document_id") {
    assert(canonicalIdentity.Document === row.result_identity.value, `${row.public_kind} selected canonical document '${canonicalIdentity.Document ?? "missing"}' instead of '${row.result_identity.value}'.`);
    await page.locator(".nd-general-sem-canonical-results-workspace").waitFor({ state: "visible", timeout });
  } else {
    assert(selectedValue === row.result_identity.value, `${row.public_kind} selected run '${selectedValue}' instead of '${row.result_identity.value}'.`);
  }
  await page.waitForTimeout(400);
  return {
    expected_option_value: expected,
    selected_option_value: selectedValue,
    canonical_export_identity: Object.keys(canonicalIdentity).length ? canonicalIdentity : null,
    passed: true,
  };
}

async function observedResultsLabels(page) {
  return page.evaluate(() => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
    const visible = (element) => {
      if (!(element instanceof HTMLElement)) return false;
      const style = getComputedStyle(element);
      return style.display !== "none" && style.visibility !== "hidden" && element.getClientRects().length > 0;
    };
    const unique = (values, limit = 120) => [...new Set(values.map(clean).filter(Boolean))].slice(0, limit);
    const select = document.querySelector(".nd-results-nav .nd-run-select select");
    const selected = select instanceof HTMLSelectElement ? select.selectedOptions[0] : null;
    const workspace = document.querySelector(".nd-results-workspace");
    return {
      selected_result: clean(selected?.textContent),
      document_tab: clean(document.querySelector(".nd-results-document .nd-document-tab")?.textContent),
      navigation: unique(Array.from(document.querySelectorAll(".nd-result-tree [role='treeitem']")).filter(visible).map((node) => node.textContent)),
      visible_headings: unique(Array.from(workspace?.querySelectorAll("h1, h2, h3") ?? []).filter(visible).map((node) => node.textContent), 40),
      visible_result_table_ids: unique(Array.from(workspace?.querySelectorAll("[data-result-table-id], [data-canonical-table-id]") ?? []).filter(visible).map((node) => node.getAttribute("data-result-table-id") ?? node.getAttribute("data-canonical-table-id"))),
    };
  });
}

async function sourceReceiptEvidence(row, staging, cache) {
  const relative = row.prior_receipt.path;
  const absolute = resolveRepoFile(relative, `${row.public_kind} source receipt`);
  assert(await exists(absolute), `${row.public_kind} source receipt is missing: ${relative}`);
  const payload = await readJson(absolute);
  const observed = jsonPointer(payload, row.prior_receipt.json_pointer);
  assert(observed !== undefined, `${row.public_kind} source receipt pointer does not resolve: ${row.prior_receipt.json_pointer}`);
  const directlyBound = observed === row.result_identity.value;
  const archiveRecovered = row.prior_receipt.identity_recovered_from_archive_not_pointer === true;
  assert(directlyBound || archiveRecovered, `${row.public_kind} source receipt pointer does not bind the declared identity and is not marked archive-recovered.`);
  if (row.public_kind === POSTHOC_KIND) {
    assert(directlyBound, "A newly supplied posthoc receipt must bind its declared identity directly; archive-recovered identity is not accepted.");
  }
  const staged = await stageArtifact({
    source: absolute,
    sourceRelative: relative,
    directory: "source-receipts",
    staging,
    cache,
  });
  return {
    ...staged,
    json_pointer: row.prior_receipt.json_pointer,
    pointer_value: pointerValueSummary(observed),
    declared_identity_directly_bound: directlyBound,
    identity_recovered_from_archive: archiveRecovered,
  };
}

async function currentScreenshot(page, staging, ordinal, kind) {
  const member = slash(path.join("screenshots", `${String(ordinal).padStart(2, "0")}-${safeFileName(kind)}-results.png`));
  const absolute = path.join(staging, ...member.split("/"));
  await fs.mkdir(path.dirname(absolute), { recursive: true });
  await page.evaluate(() => {
    for (const selector of [".nd-workspace", ".nd-results-document", ".nd-results-workspace"]) {
      const element = document.querySelector(selector);
      if (element instanceof HTMLElement) element.scrollTop = 0;
    }
  });
  await page.screenshot({ path: absolute, animations: "disabled" });
  const stat = await fs.stat(absolute);
  return { member, sha256: await fileSha256(absolute), size_bytes: stat.size };
}

async function writeMethodReceipt(staging, ordinal, payload) {
  const member = slash(path.join("receipts", `${String(ordinal).padStart(2, "0")}-${safeFileName(payload.method_kind)}.json`));
  const absolute = path.join(staging, ...member.split("/"));
  await writeJsonNew(absolute, payload);
  const stat = await fs.stat(absolute);
  return { member, sha256: await fileSha256(absolute), size_bytes: stat.size, status: payload.status, method_kind: payload.method_kind };
}

function blockedPosthocReceipt(row, inventoryPath) {
  return {
    schema_version: 1,
    suite_id: "quickpls_v255_frozen_archive_reopen_crawler_v1",
    target_release: TARGET_RELEASE,
    generated_at: new Date().toISOString(),
    status: "blocked",
    method_kind: row.public_kind,
    declared_identity: null,
    source_inventory: inventoryPath,
    archive: null,
    screenshot: null,
    source_receipt: null,
    observed_results_labels: null,
    new_scientific_run_required: true,
    failure_code: "posthoc_new_run_archive_and_receipt_required",
    failure: "Posthoc technical minimum sample-size publication evidence requires a separately supplied new-run archive and receipt whose JSON pointer directly binds the new result identity.",
  };
}

async function crawlMethod({ page, row, ordinal, staging, inventoryPath, timeout, cache }) {
  validateResultIdentity(row.result_identity, row.public_kind);
  const archiveAbsolute = resolveRepoFile(row.archive_path, `${row.public_kind} archive`);
  assert(await exists(archiveAbsolute), `${row.public_kind} archive is missing: ${row.archive_path}`);
  const archive = await stageArtifact({
    source: archiveAbsolute,
    sourceRelative: row.archive_path,
    directory: "archives",
    staging,
    cache,
  });
  const sourceReceipt = await sourceReceiptEvidence(row, staging, cache);
  await openArchive(page, archiveAbsolute, timeout);
  const identityVerification = await navigateToDeclaredResult(page, row, timeout);
  const labels = await observedResultsLabels(page);
  assert(labels.selected_result, `${row.public_kind} exposed no selected Results label.`);
  assert(labels.document_tab, `${row.public_kind} exposed no Results document-tab label.`);
  const screenshot = await currentScreenshot(page, staging, ordinal, row.public_kind);
  return {
    schema_version: 1,
    suite_id: "quickpls_v255_frozen_archive_reopen_crawler_v1",
    target_release: TARGET_RELEASE,
    generated_at: new Date().toISOString(),
    status: "verified_current_ui_capture",
    method_kind: row.public_kind,
    declared_identity: row.result_identity,
    scientific_identity: row.scientific_identity ?? null,
    identity_verification: identityVerification,
    archive,
    screenshot,
    source_receipt: sourceReceipt,
    source_release: row.source_release ?? null,
    source_inventory: inventoryPath,
    observed_results_labels: labels,
    current_ui_capture_required: false,
    new_scientific_run_required: false,
  };
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  const staging = path.resolve(args["staging-dir"]);
  assert(inside(RESULTS_ROOT, staging) && staging !== RESULTS_ROOT, "--staging-dir must be a new child directory below validation/results.");
  assert(!await exists(staging), `Refusing to reuse or overwrite staging directory: ${staging}`);
  await fs.mkdir(staging, { recursive: false });

  const inventoryAbsolute = args.inventory
    ? resolveRepoFile(args.inventory, "--inventory")
    : DEFAULT_INVENTORY;
  assert(await exists(inventoryAbsolute), `Reusable archive inventory is missing: ${inventoryAbsolute}`);
  const inventoryRelative = slash(path.relative(ROOT, inventoryAbsolute));
  const inventory = await readJson(inventoryAbsolute);
  const rows = validateInventory(inventory);
  const posthocSupplement = await loadPosthocSupplement(args["posthoc-supplement"]);
  const connection = await connectToIsolatedPackagedPage(args.endpoint, args.timeout);
  const rendererErrors = observeRendererErrors(connection.page);
  const cache = new Map();
  const methodReceipts = [];
  const failures = [];

  for (let index = 0; index < rows.length; index += 1) {
    const inventoryRow = rows[index];
    const ordinal = index + 1;
    const row = inventoryRow.public_kind === POSTHOC_KIND && posthocSupplement
      ? posthocSupplement
      : inventoryRow;
    let receipt;
    if (row.public_kind === POSTHOC_KIND && row.new_scientific_run_required === true) {
      receipt = blockedPosthocReceipt(row, inventoryRelative);
    } else {
      try {
        receipt = await crawlMethod({
          page: connection.page,
          row,
          ordinal,
          staging,
          inventoryPath: inventoryRelative,
          timeout: args.timeout,
          cache,
        });
      } catch (error) {
        receipt = {
          schema_version: 1,
          suite_id: "quickpls_v255_frozen_archive_reopen_crawler_v1",
          target_release: TARGET_RELEASE,
          generated_at: new Date().toISOString(),
          status: "failed",
          method_kind: row.public_kind,
          declared_identity: row.result_identity ?? null,
          source_inventory: inventoryRelative,
          archive: row.archive_path ? { source_path: row.archive_path } : null,
          screenshot: null,
          source_receipt: row.prior_receipt ?? null,
          observed_results_labels: null,
          new_scientific_run_required: false,
          failure_code: "archive_reopen_or_result_capture_failed",
          failure: error instanceof Error ? error.message : String(error),
        };
      }
    }
    const artifact = await writeMethodReceipt(staging, ordinal, receipt);
    methodReceipts.push(artifact);
    if (receipt.status !== "verified_current_ui_capture") {
      failures.push({ method_kind: receipt.method_kind, status: receipt.status, failure_code: receipt.failure_code, failure: receipt.failure });
    }
  }

  try {
    await rendererErrors.settle();
  } catch (error) {
    failures.push({
      method_kind: null,
      status: "failed",
      failure_code: "renderer_error_observation_did_not_settle",
      failure: error instanceof Error ? error.message : String(error),
    });
  }
  const consoleErrors = [...rendererErrors.errors];
  rendererErrors.stop();
  if (consoleErrors.length > 0) {
    failures.push({
      method_kind: null,
      status: "failed",
      failure_code: "renderer_console_error",
      failure: `Renderer errors were observed: ${JSON.stringify(consoleErrors)}`,
    });
  }

  const stagedFiles = (await fs.readdir(staging, { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile())
    .map((entry) => slash(path.relative(staging, path.join(entry.parentPath ?? entry.path, entry.name))))
    .sort();
  const manifest = {
    schema_version: 1,
    suite_id: "quickpls_v255_frozen_archive_reopen_crawler_v1",
    target_release: TARGET_RELEASE,
    generated_at: new Date().toISOString(),
    status: failures.length ? "failed" : "passed",
    source_inventory: {
      path: inventoryRelative,
      sha256: await fileSha256(inventoryAbsolute),
    },
    posthoc_supplement: args["posthoc-supplement"] ?? null,
    serial: true,
    maximum_concurrent_archives: 1,
    method_receipts: methodReceipts,
    console_errors: consoleErrors,
    named_evidence_observations: [],
    staged_members_before_manifest: stagedFiles,
    failures,
    cdp_preflight: {
      endpoint_origin: new URL(args.endpoint).origin,
      packaged_page: connection.page_state,
      enumerated_pages: connection.enumerated_pages,
    },
    process_safety: {
      wrapper_owns_exact_pid_lifecycle: true,
      driver_launches_app_processes: false,
      driver_terminates_app_processes: false,
      driver_closes_browser_page_or_context: false,
    },
  };
  if (failures.length === 0 && methodReceipts.length >= 2) {
    const firstReceipt = await readJson(path.join(staging, ...methodReceipts[0].member.split("/")));
    manifest.named_evidence_observations.push(await namedObservationFromReceipt({
      staging,
      receiptArtifact: methodReceipts[0],
      caseId: "cross_method:observability:machine-readable observation",
      observed: {
        method_kind: firstReceipt.method_kind,
        selected_result: firstReceipt.observed_results_labels.selected_result,
        document_tab: firstReceipt.observed_results_labels.document_tab,
        navigation: firstReceipt.observed_results_labels.navigation,
        visible_headings: firstReceipt.observed_results_labels.visible_headings,
        visible_result_table_ids: firstReceipt.observed_results_labels.visible_result_table_ids,
      },
    }));
    manifest.named_evidence_observations.push(await namedObservationFromReceipt({
      staging,
      receiptArtifact: methodReceipts[1],
      caseId: "cross_method:observability:zero unexplained skip",
      observed: {
        expected_method_receipts: rows.length,
        verified_method_receipts: methodReceipts.filter((item) => item.status === "verified_current_ui_capture").length,
        unexplained_skips: failures.length,
      },
    }));
  }
  const manifestPath = path.join(staging, "receipts", "v255-frozen-archive-reopen-crawler.json");
  await writeJsonNew(manifestPath, manifest);

  return {
    exit_code: failures.length ? 1 : 0,
    summary: {
      passed: failures.length === 0,
      target_release: TARGET_RELEASE,
      staging_dir: slash(path.relative(ROOT, staging)),
      verified_methods: methodReceipts.filter((entry) => entry.status === "verified_current_ui_capture").length,
      failed_or_blocked_methods: failures,
      aggregate_receipt: slash(path.relative(ROOT, manifestPath)),
      note: "The driver intentionally leaves the isolated packaged QuickPLS process running for its exact-PID wrapper to own and terminate.",
    },
  };
}

let outcome;
try {
  outcome = await run();
} catch (error) {
  outcome = {
    exit_code: 1,
    summary: {
      passed: false,
      target_release: TARGET_RELEASE,
      fatal: error instanceof Error ? error.message : String(error),
      note: "No app process was launched or terminated by this driver.",
    },
  };
}

// Do not call browser.close(): for a local connectOverCDP Browser object that
// sends Browser.close and would violate the wrapper-owned PID safety boundary.
// All filesystem and screenshot work is already awaited here; exiting this
// driver process merely drops its transport sockets.
process.stdout.write(`${JSON.stringify(outcome.summary, null, 2)}\n`, () => {
  process.exit(outcome.exit_code);
});
