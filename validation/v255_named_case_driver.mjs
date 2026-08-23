#!/usr/bin/env node
/**
 * Manifest-driven QuickPLS 2.55 named-case evidence driver.
 *
 * This driver attaches to one wrapper-owned packaged candidate. It never starts,
 * terminates, or closes QuickPLS. A curated manifest may select only allowlisted
 * UI operations and exact observable assertions; arbitrary JavaScript is not a
 * supported step. Every passing case receives one current PNG and one additive
 * named_evidence_observations record. Unsupported or incomplete cases fail closed.
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
const RESULTS = path.join(ROOT, "validation", "results");
const DEFAULT_INDEX = path.join(ROOT, "validation", "v255_named_evidence_index.json");
const DEFAULT_MANIFEST = path.join(ROOT, "validation", "v255_named_case_manifest.json");
const NAMED_SEM_DATASET = path.join(ROOT, "validation", "fixtures", "v255", "named-sem-evidence.csv");
const FILE_DIALOG_HELPER = path.join(ROOT, "validation", "windows_native_owned_file_dialog.py");
const ARCHIVE_IDENTITY_HELPER = path.join(ROOT, "validation", "v255_named_archive_identity.py");
const TARGET_RELEASE = "2.55.0";
const SUITE_ID = "quickpls_v255_named_case_driver_v1";
const MANIFEST_SUITE_ID = "quickpls_v255_named_case_manifest_v1";
const RENDERER_ERROR_SETTLE_MS = 250;
const ALLOWED_VIEWS = new Set(["welcome", "home", "data", "models", "model", "runs", "results"]);
const ALLOWED_STATES = new Set(["attached", "detached", "visible", "hidden"]);
const ARCHIVE_SUPPLEMENT_PUBLIC_KINDS = new Set(["pls_algorithm", "pls_bootstrap", "cbsem"]);
const SAFE_RESULT_ID = /^[A-Za-z0-9_.:-]+$/u;

function assert(condition, message) { if (!condition) throw new Error(message); }
function compact(value) { return String(value ?? "").replace(/\s+/g, " ").trim(); }
function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}
function slash(value) { return String(value).split(path.sep).join("/"); }
function surfaceForView(view) {
  if (view === "welcome" || view === "home") return "launcher";
  if (view === "models" || view === "model") return "model";
  if (view === "runs" || view === "results") return "results";
  return "data";
}
function safeFileName(value) {
  const result = String(value).toLocaleLowerCase("en-US").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return result.slice(0, 100) || "case";
}
function sha256(value) { return crypto.createHash("sha256").update(value).digest("hex"); }
async function fileSha256(file) { return sha256(await fs.readFile(file)); }
async function exists(file) { return fs.stat(file).then(() => true, () => false); }
async function readJson(file) {
  const value = JSON.parse(await fs.readFile(file, "utf8"));
  assert(value && typeof value === "object" && !Array.isArray(value), `JSON root must be an object: ${file}`);
  return value;
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
async function waitForOpenedProjectPath(page, archive, timeout) {
  await page.waitForFunction((target) => {
    const normalize = (value) => String(value ?? "")
      .trim()
      .replace(/\//g, "\\")
      .replace(/\\+/g, "\\")
      .toLocaleLowerCase("en-US");
    const displayed = document.querySelector(".nd-document-context span")?.textContent ?? "";
    return normalize(displayed) === normalize(target);
  }, archive, { timeout });
}
async function runJsonProcess(executable, argumentsList, timeout) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, argumentsList, { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`Helper timed out after ${timeout}ms: ${argumentsList[0]}`));
    }, timeout);
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", (error) => { clearTimeout(timer); reject(error); });
    child.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0) return reject(new Error(`Helper failed (${code}): ${compact(stderr || stdout)}`));
      try { resolve(JSON.parse(stdout)); } catch (error) { reject(new Error(`Helper emitted invalid JSON: ${String(error)}`)); }
    });
  });
}
function stable(value) {
  if (Array.isArray(value)) return value.map(stable);
  if (value && typeof value === "object") return Object.fromEntries(Object.keys(value).sort().map((key) => [key, stable(value[key])]));
  return value;
}
function deepEqual(left, right) { return JSON.stringify(stable(left)) === JSON.stringify(stable(right)); }
function resultTableSelector(tableId) {
  assert(typeof tableId === "string" && SAFE_RESULT_ID.test(tableId), `Unsafe result table ID: ${tableId}`);
  return `[data-result-table-id="${tableId}"], [data-canonical-table-id="${tableId}"]`;
}
function resultTreeItemSelector(tableId) {
  assert(typeof tableId === "string" && SAFE_RESULT_ID.test(tableId), `Unsafe result tree item ID: ${tableId}`);
  return `[data-result-tree-item-id="canonical:table:${tableId}"], [data-result-tree-item-id="${tableId}"]`;
}
async function waitForSingleVisibleLocator(page, selector, timeout, { allowAbsent = false } = {}) {
  assert(typeof selector === "string" && selector, "Visible locator selection requires a selector.");
  assert(Number.isInteger(timeout) && timeout >= 0, "Visible locator selection requires a non-negative integer timeout.");
  const deadline = Date.now() + timeout;
  let totalCount = 0;
  let visibleCount = 0;
  do {
    const matches = page.locator(selector);
    totalCount = await matches.count();
    const visibleMatches = matches.filter({ visible: true });
    visibleCount = await visibleMatches.count();
    if (visibleCount === 1) return visibleMatches;
    assert(visibleCount < 2,
      `Visible locator selection is ambiguous for ${selector}: ${totalCount} total, ${visibleCount} visible.`);
    if (allowAbsent) return null;
    if (Date.now() < deadline) await page.waitForTimeout(Math.min(50, Math.max(1, deadline - Date.now())));
  } while (Date.now() < deadline);
  throw new Error(`Visible locator selection timed out for ${selector}: ${totalCount} total, ${visibleCount} visible.`);
}

function parseArgs(argv) {
  const allowed = new Set(["endpoint", "manifest", "index", "evidence-dir", "candidate-name", "candidate-pid", "candidate-path", "python", "timeout-ms"]);
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
  for (const key of ["endpoint", "evidence-dir", "candidate-name", "candidate-pid", "candidate-path", "python"]) assert(values[key], `--${key} is required.`);
  assert(new Set(["portable", "installed"]).has(values["candidate-name"]), "--candidate-name must be portable or installed.");
  const candidatePid = Number(values["candidate-pid"]);
  assert(Number.isSafeInteger(candidatePid) && candidatePid > 0, "--candidate-pid must be a positive process ID.");
  assert(path.isAbsolute(values["candidate-path"]), "--candidate-path must be an absolute executable path.");
  const candidatePath = path.resolve(values["candidate-path"]);
  assert(path.extname(candidatePath).toLocaleLowerCase("en-US") === ".exe", "--candidate-path must name a Windows executable.");
  const timeout = Number(values["timeout-ms"] ?? 60_000);
  assert(Number.isInteger(timeout) && timeout >= 5_000 && timeout <= 300_000, "--timeout-ms must be from 5000 through 300000.");
  return {
    ...values,
    manifest: path.resolve(values.manifest ?? DEFAULT_MANIFEST),
    index: path.resolve(values.index ?? DEFAULT_INDEX),
    evidenceDir: path.resolve(values["evidence-dir"]),
    candidateName: values["candidate-name"],
    candidatePid,
    candidatePath,
    python: path.resolve(values.python),
    timeout,
  };
}

function assertLoopback(endpoint) {
  const parsed = new URL(endpoint);
  assert(["http:", "https:", "ws:", "wss:"].includes(parsed.protocol), "CDP endpoint protocol is unsupported.");
  assert(["127.0.0.1", "localhost", "::1", "[::1]"].includes(parsed.hostname), "CDP endpoint must be loopback-only.");
  assert(parsed.port, "CDP endpoint requires an explicit port.");
}

function resolveRepoPath(value, label) {
  assert(typeof value === "string" && value.trim(), `${label} must be a non-empty path.`);
  const absolute = path.isAbsolute(value) ? path.resolve(value) : path.resolve(ROOT, value);
  assert(inside(ROOT, absolute), `${label} escapes the repository: ${value}`);
  return absolute;
}

function expectedCandidate(caseId, contract) {
  const selection = contract.candidate_selection;
  assert(selection?.default === "portable" && selection?.overrides && typeof selection.overrides === "object", "Index candidate selection contract is invalid.");
  return selection.overrides[caseId] ?? selection.default;
}

function freshSpecializedExpected(entry, route) {
  const counts = {
    specific_indirect_count: 0,
    interaction_effect_count: 0,
    conditional_slope_count: 0,
    conditional_probe_contracts: [],
    three_way_effect_count: 0,
    three_way_conditional_effect_count: 0,
    three_way_simple_slope_count: 0,
    three_way_probe_contracts: [],
    conditional_indirect_count: 0,
    moderated_mediation_index_count: 0,
    higher_order_stage_count: 0,
    ...route.result_counts,
  };
  const fixtureVisible = route.advanced_parameter_revision !== true;
  return {
    fixture: fixtureVisible ? route.fixture : null,
    fixture_model_id: fixtureVisible ? `v255-${route.fixture}` : null,
    fixture_constructs: fixtureVisible ? route.fixture_receipt.constructs : null,
    fixture_derived_terms: fixtureVisible ? route.fixture_receipt.derived_terms : null,
    fixture_paths: fixtureVisible ? route.fixture_receipt.paths : null,
    selected_method: route.method,
    selected_inference: route.inference,
    selected_moderated_stage: route.moderated_stage ?? null,
    route_expectations_passed: true,
    result_identity_expectations_passed: true,
    selected_result_id: null,
    selected_result_id_present: true,
    table_id: route.table_id,
    table_shape_passed: true,
    header_expectations_passed: true,
    row_expectations_passed: true,
    navigation_expectations_passed: true,
    source_archive_identity: null,
    smoke_snapshot: {
      model: {
        ordinary_construct_count: route.model.ordinary_construct_count,
        common_factor_count: route.model.common_factor_count,
        structural_relation_count: route.model.structural_relation_count,
        interaction_orders: route.model.interaction_orders,
        higher_order_measurement_types: route.model.higher_order_measurement_types,
        advanced_equality_labels: route.advanced_parameter_revision === true ? ["V255Evidence"] : [],
      },
      canonical_result: {
        document_id_present: true,
        run_id_present: true,
        method_version: route.result.method_version,
        primary_cell_id: route.result.primary_cell_id,
        execution_cell_id: route.result.execution_cell_id,
        capability_cell_ids: [...route.result.capability_cell_ids].sort(),
        selected_table: { id: route.table_id, rows_nonempty: true, columns_nonempty: true },
        ...counts,
      },
      legacy_result: null,
    },
    advanced_parameter_revision: route.advanced_parameter_revision === true ? {
      parameter_id_present: true,
      parameter_count: 2,
      before_digest_present: true,
      after_digest_present: true,
      equality_label: "V255Evidence",
      stable_parameter_id: true,
      changed_authority: true,
    } : null,
  };
}

function archiveSpecializedExpected(route) {
  return {
    fixture: null,
    fixture_model_id: null,
    fixture_constructs: null,
    fixture_derived_terms: null,
    fixture_paths: null,
    selected_method: null,
    selected_inference: null,
    selected_moderated_stage: null,
    route_expectations_passed: true,
    result_identity_expectations_passed: true,
    selected_result_id: route.selected_result_value,
    selected_result_id_present: true,
    table_id: route.table_id,
    table_shape_passed: true,
    header_expectations_passed: true,
    row_expectations_passed: true,
    navigation_expectations_passed: true,
    source_archive_identity: route.archive_identity,
    smoke_snapshot: null,
    advanced_parameter_revision: null,
  };
}

function materializeManifestCase(entry) {
  if (!entry?.route) return entry;
  const route = entry.route;
  assert(route && typeof route === "object", `${entry.id} route must be an object.`);
  if (route.kind === "archive_result") {
    assert(typeof route.archive === "string" && typeof route.result_id === "string" && typeof route.table_id === "string", `${entry.id} archive route is incomplete.`);
    assert(/^[a-f0-9]{64}$/u.test(route.archive_sha256 ?? ""), `${entry.id} archive route lacks an exact lowercase SHA-256.`);
    assert(route.archive_identity && route.archive_identity.result_id === route.result_id.replace(/^canonical:/u, "") && route.archive_identity.table_id === route.table_id, `${entry.id} archive identity is not bound to the selected result/table.`);
    const query = {
      kind: "specialized_result",
      table_id: route.table_id,
      require_exact_selected_result: true,
      header_contains: route.header_contains,
      row_contains: route.row_contains,
      navigation_contains: route.navigation_contains,
      result_contains: route.result_contains,
    };
    return {
      ...entry,
      steps: [
        { action: "goto_packaged" },
        { action: "open_archive", path: route.archive, expected_sha256: route.archive_sha256 },
        { action: "inspect_archive_identity", path: route.archive, result_id: route.result_id, table_id: route.table_id, expected: route.archive_identity },
        { action: "select_result", value: route.selected_result_value },
        { action: "select_result_table", table_id: route.table_id },
      ],
      assertion: { id: `${entry.operation}:${entry.id}`, query, expected: archiveSpecializedExpected(route) },
      screenshot: { selector: resultTableSelector(route.table_id) },
    };
  }
  if (route.kind === "fresh_cfa_bootstrap_result") {
    assert(route.fixture === "cfa" && route.method === "cbsem" && route.inference === "case_bootstrap", `${entry.id} exact CFA route identity is invalid.`);
    assert(typeof route.table_id === "string" && route.result && route.model, `${entry.id} exact CFA route is incomplete.`);
    const query = {
      kind: "cfa_compatibility_result",
      table_id: route.table_id,
      route_contains: route.route_contains,
      header_contains: route.header_contains,
      row_contains: route.row_contains,
      minimum_rows: route.minimum_rows ?? 1,
      minimum_columns: route.minimum_columns ?? 2,
    };
    return {
      ...entry,
      steps: [
        { action: "goto_packaged" },
        { action: "create_project", name: `QuickPLS 2.55 evidence ${entry.id}` },
        { action: "load_named_sem_fixture", fixture: route.fixture },
        { action: "prepare_calculation_revision", method: "cbsem", inference: "point" },
        { action: "run_calculation", method: route.method, inference: route.inference, bootstrap_samples: route.bootstrap_samples ?? 500, route_contains: route.route_contains, requires_requested_revision: false, completion_timeout_ms: route.completion_timeout_ms ?? 300_000 },
        { action: "select_result_table", table_id: route.table_id },
      ],
      assertion: {
        id: `${entry.operation}:${entry.id}`,
        query,
        expected: {
          fixture: "cfa",
          selected_method: "cbsem",
          selected_inference: "case_bootstrap",
          route_expectations_passed: true,
          table_id: route.table_id,
          table_shape_passed: true,
          header_expectations_passed: true,
          row_expectations_passed: true,
          canonical_document_id_present: true,
          canonical_method_version: route.result.method_version,
          primary_cell_id: route.result.primary_cell_id,
          execution_cell_id: route.result.execution_cell_id,
          capability_cell_ids: [...route.result.capability_cell_ids].sort(),
          model: route.model,
        },
      },
      screenshot: { selector: resultTableSelector(route.table_id) },
    };
  }
  assert(route.kind === "fresh_sem_result", `${entry.id} uses unsupported route kind ${route.kind}.`);
  assert(typeof route.fixture === "string" && typeof route.method === "string" && typeof route.table_id === "string", `${entry.id} fresh SEM route is incomplete.`);
  assert(route.fixture_receipt && route.model && route.result, `${entry.id} fresh SEM route lacks exact fixture/model/result identity.`);
  if (route.archive_supplement_public_kind !== undefined) {
    assert(entry.candidate === "portable", `${entry.id} archive supplement must be collected from the portable candidate.`);
    assert(ARCHIVE_SUPPLEMENT_PUBLIC_KINDS.has(route.archive_supplement_public_kind), `${entry.id} archive supplement public method kind is unsupported.`);
    assert(route.archive_supplement_public_kind === route.method, `${entry.id} archive supplement public method kind must equal its executed method.`);
  }
  const prepareCalculationRevision = {
    action: "prepare_calculation_revision",
    method: route.method,
    inference: route.inference,
    bootstrap_samples: route.bootstrap_samples,
  };
  const requiresRequestedRevision = route.moderated_stage === "first_stage"
    || route.moderated_stage === "second_stage";
  const steps = [
    { action: "goto_packaged" },
    { action: "create_project", name: `QuickPLS 2.55 evidence ${entry.id}` },
    { action: "load_named_sem_fixture", fixture: route.fixture },
    ...(requiresRequestedRevision ? [] : [prepareCalculationRevision]),
  ];
  if (route.advanced_parameter_revision === true) {
    steps.push({ action: "exercise_advanced_parameter_revision" }, { action: "save_and_reopen_case_revision" });
  }
  steps.push({
    action: "run_calculation",
    method: route.method,
    inference: route.inference,
    bootstrap_samples: route.bootstrap_samples,
    moderated_stage: route.moderated_stage,
    route_contains: route.route_contains,
    requires_requested_revision: requiresRequestedRevision,
    completion_timeout_ms: route.completion_timeout_ms ?? 300_000,
  }, { action: "select_result_table", table_id: route.table_id });
  if (route.archive_supplement_public_kind !== undefined) {
    steps.push({
      action: "save_result_archive_supplement",
      public_kind: route.archive_supplement_public_kind,
      table_id: route.table_id,
      method_version: route.result.method_version,
      capability_cell_ids: [...route.result.capability_cell_ids].sort(),
    });
  }
  const query = {
    kind: "specialized_result",
    table_id: route.table_id,
    include_smoke_snapshot: true,
    include_advanced_parameter_revision: route.advanced_parameter_revision === true,
    route_contains: route.route_contains,
    result_contains: route.result_contains,
    header_contains: route.header_contains,
    row_contains: route.row_contains,
    navigation_contains: route.navigation_contains,
    minimum_rows: route.minimum_rows ?? 1,
    minimum_columns: route.minimum_columns ?? 2,
  };
  return {
    ...entry,
    steps,
    assertion: { id: `${entry.operation}:${entry.id}`, query, expected: freshSpecializedExpected(entry, route) },
    screenshot: { selector: resultTableSelector(route.table_id) },
  };
}

function validateManifest(manifest, index, candidateName) {
  assert(manifest.schema_version === 1 && manifest.suite_id === MANIFEST_SUITE_ID && manifest.target_release === TARGET_RELEASE, "Named-case manifest identity is invalid.");
  assert(manifest.status === "ready" && manifest.coverage_status === "complete", "Named-case manifest must provide a complete executable route for every frozen case.");
  assert(Array.isArray(manifest.cases) && manifest.cases.length > 0, "Named-case manifest has no concrete cases.");
  const indexEntries = new Map((index.entries ?? []).map((entry) => [entry.id, entry]));
  const contract = index.collector_contract;
  const suppliedByFixedDrivers = manifest.supplied_by_fixed_drivers;
  const pendingCases = manifest.pending_cases;
  assert(Array.isArray(suppliedByFixedDrivers) && suppliedByFixedDrivers.every((id) => typeof id === "string" && id), "Manifest supplied_by_fixed_drivers must be exact case IDs.");
  assert(Array.isArray(pendingCases) && pendingCases.every((entry) => entry && typeof entry.id === "string" && typeof entry.reason === "string" && entry.reason.trim()), "Every pending manifest case needs an exact ID and non-empty reason.");
  const curatedIds = manifest.cases.map((entry) => entry?.id);
  const pendingIds = pendingCases.map((entry) => entry.id);
  const partition = [...suppliedByFixedDrivers, ...curatedIds, ...pendingIds];
  assert(partition.length === indexEntries.size && new Set(partition).size === partition.length, "Fixed, curated, and pending manifest cases must form a duplicate-free frozen-case partition.");
  assert(partition.every((id) => indexEntries.has(id)) && new Set(partition).size === indexEntries.size, "Manifest case partition does not exactly equal the frozen named-evidence index.");
  assert(manifest.coverage?.frozen_case_count === indexEntries.size
    && manifest.coverage?.fixed_driver_case_count === suppliedByFixedDrivers.length
    && manifest.coverage?.curated_case_count === manifest.cases.length
    && manifest.coverage?.pending_case_count === pendingCases.length,
  "Manifest coverage counts do not match its exact case partition.");
  const selectedCases = manifest.cases.filter((entry) => entry?.candidate === candidateName).map(materializeManifestCase);
  assert(selectedCases.length > 0, `Named-case manifest has no ${candidateName} cases.`);
  assert(new Set(selectedCases.map((entry) => entry.id)).size === selectedCases.length, `Named-case manifest duplicates a ${candidateName} case.`);
  for (const entry of selectedCases) {
    const indexed = indexEntries.get(entry.id);
    assert(indexed, `Manifest case is not in the frozen 55-case index: ${entry.id}`);
    assert(entry.candidate === expectedCandidate(entry.id, contract), `${entry.id} must run on ${expectedCandidate(entry.id, contract)}, not ${entry.candidate}.`);
    const operation = contract.operation_by_group?.[indexed.group];
    assert(entry.operation === operation, `${entry.id} operation must be ${operation}.`);
    assert(entry.assertion?.id === `${operation}:${entry.id}`, `${entry.id} assertion ID is not exact.`);
    assert(entry.assertion && Object.hasOwn(entry.assertion, "expected") && entry.assertion.expected !== null, `${entry.id} requires a non-null expected value.`);
    assert(entry.assertion.query && typeof entry.assertion.query === "object", `${entry.id} requires one final observable query.`);
    assert(Array.isArray(entry.steps) && entry.steps.length > 0, `${entry.id} requires concrete UI steps.`);
  }
  return { selectedCases, indexEntries, contract };
}

function createDialogHelper({ python, mode, target, allowedRoot, extension, candidatePid, candidatePath }) {
  assert(Number.isSafeInteger(candidatePid) && candidatePid > 0, "Native dialog helper requires the exact positive candidate PID.");
  assert(path.isAbsolute(candidatePath) && path.extname(candidatePath).toLocaleLowerCase("en-US") === ".exe", "Native dialog helper requires the exact absolute candidate executable.");
  const child = spawn(python, [
    FILE_DIALOG_HELPER,
    "--mode", mode,
    "--target", target,
    "--allowed-root", allowedRoot,
    "--window-title", "QuickPLS",
    "--owner-pid", String(candidatePid),
    "--owner-executable", candidatePath,
    "--timeout-seconds", "90",
    "--extension", extension,
  ], { cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  let pending = "";
  let stderr = "";
  let readyResolve;
  let completeResolve;
  let readySettled = false;
  let completeSettled = false;
  const ready = new Promise((resolve) => { readyResolve = resolve; });
  const completed = new Promise((resolve) => { completeResolve = resolve; });
  const settleReady = (event) => { if (!readySettled) { readySettled = true; readyResolve(event); } };
  const settleComplete = (event) => { if (!completeSettled) { completeSettled = true; completeResolve(event); } };
  const consume = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      if (event.event === "ready") settleReady(event);
      if (event.event === "complete") { if (!event.passed) settleReady(event); settleComplete(event); }
    } catch (error) {
      const event = { event: "complete", passed: false, message: String(error), line };
      settleReady(event);
      settleComplete(event);
    }
  };
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    pending += chunk;
    const lines = pending.split(/\r?\n/);
    pending = lines.pop() ?? "";
    lines.forEach(consume);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.on("error", (error) => { const event = { event: "complete", passed: false, message: error.message }; settleReady(event); settleComplete(event); });
  child.on("close", (code, signal) => {
    if (pending.trim()) consume(pending);
    const event = { event: "complete", passed: false, code, signal, stderr: compact(stderr) };
    settleReady(event);
    settleComplete(event);
  });
  return { ready, completed, stop: () => { if (!child.killed) child.kill(); } };
}

async function observe(page, query, evidenceDir, context = {}) {
  assert(query && typeof query === "object", "Observation query must be an object.");
  const kind = query.kind;
  if (kind === "viewport") return page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight, device_pixel_ratio: window.devicePixelRatio }));
  if (kind === "candidate_surface") {
    return page.evaluate(() => ({
      origin: window.location.origin,
      native_shell_visible: Boolean(document.querySelector('.nd-app[data-native-desktop-shell="true"]')),
      viewport: { width: window.innerWidth, height: window.innerHeight },
    }));
  }
  if (kind === "active_element") {
    return page.evaluate(() => {
      const element = document.activeElement;
      return element instanceof HTMLElement ? {
        tag: element.tagName.toLocaleLowerCase("en-US"),
        aria_label: String(element.getAttribute("aria-label") ?? "").replace(/\s+/g, " ").trim(),
        text: String(element.textContent ?? "").replace(/\s+/g, " ").trim(),
      } : null;
    });
  }
  if (kind === "calculation_dialog") {
    return page.evaluate(() => {
      const dialog = document.querySelector(".nd-dialog-calculation");
      const title = document.querySelector("#nd-dialog-title");
      return {
        visible: dialog instanceof HTMLElement && dialog.offsetParent !== null,
        role: dialog?.getAttribute("role") ?? "",
        aria_modal: dialog?.getAttribute("aria-modal") ?? "",
        title: String(title?.textContent ?? "").replace(/\s+/g, " ").trim(),
      };
    });
  }
  if (kind === "result_table") {
    assert(typeof query.table_id === "string" && SAFE_RESULT_ID.test(query.table_id), "result_table query requires a safe table_id.");
    const select = page.locator(".nd-results-nav .nd-run-select select");
    const table = await waitForSingleVisibleLocator(page, resultTableSelector(query.table_id), context.timeout);
    const selectedResultId = await select.inputValue();
    const tableId = await table.getAttribute("data-result-table-id") ?? await table.getAttribute("data-canonical-table-id");
    const rowCount = await table.locator("tbody tr").count();
    const headerCount = await table.locator("thead th").count();
    return {
      selected_result_id: selectedResultId,
      table_id: tableId,
      visible: await table.isVisible().catch(() => false),
      nonempty_rows: rowCount > 0,
      nonempty_columns: headerCount > 0,
    };
  }
  if (kind === "specialized_result") {
    assert(typeof query.table_id === "string" && SAFE_RESULT_ID.test(query.table_id), "specialized_result query requires a safe table_id.");
    const table = await waitForSingleVisibleLocator(page, resultTableSelector(query.table_id), context.timeout);
    const select = page.locator(".nd-results-nav .nd-run-select select");
    const rows = await table.locator("tbody tr").allTextContents();
    const headers = await table.locator("thead th").allTextContents();
    const navigation = await page.locator(".nd-result-tree [role='treeitem']").allTextContents();
    const routeText = context.lastCalculation?.routeText ?? "";
    const resultText = compact(await page.locator(".nd-results-workspace").textContent());
    const smokeSnapshot = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
    const rowText = rows.map(compact);
    const headerText = headers.map(compact);
    const navigationText = navigation.map(compact);
    const includesEvery = (source, expected) => (expected ?? []).every((token) => source.some((value) => value.includes(token)));
    return {
      fixture: context.activeFixture?.fixture ?? null,
      fixture_model_id: context.activeFixture?.modelId ?? null,
      fixture_constructs: context.activeFixture?.constructs ?? null,
      fixture_derived_terms: context.activeFixture?.derivedTerms ?? null,
      fixture_paths: context.activeFixture?.paths ?? null,
      selected_method: context.lastCalculation?.method ?? null,
      selected_inference: context.lastCalculation?.inference ?? null,
      selected_moderated_stage: context.lastCalculation?.moderatedStage ?? null,
      route_expectations_passed: (query.route_contains ?? []).every((token) => routeText.includes(token)),
      result_identity_expectations_passed: (query.result_contains ?? []).every((token) => resultText.includes(token)),
      selected_result_id: query.require_exact_selected_result === true ? await select.inputValue() : null,
      selected_result_id_present: Boolean(await select.inputValue()),
      table_id: await table.getAttribute("data-result-table-id") ?? await table.getAttribute("data-canonical-table-id"),
      table_shape_passed: rows.length >= Number(query.minimum_rows ?? 1) && headers.length >= Number(query.minimum_columns ?? 1),
      header_expectations_passed: includesEvery(headerText, query.header_contains),
      row_expectations_passed: includesEvery(rowText, query.row_contains),
      navigation_expectations_passed: includesEvery(navigationText, query.navigation_contains),
      source_archive_identity: context.archiveIdentity ?? null,
      smoke_snapshot: query.include_smoke_snapshot === true && smokeSnapshot ? {
        model: smokeSnapshot.model ? {
          ordinary_construct_count: smokeSnapshot.model.ordinary_construct_count,
          common_factor_count: smokeSnapshot.model.common_factor_count,
          structural_relation_count: smokeSnapshot.model.structural_relation_count,
          interaction_orders: smokeSnapshot.model.interaction_orders,
          higher_order_measurement_types: smokeSnapshot.model.higher_order_measurement_types,
          advanced_equality_labels: smokeSnapshot.model.advanced_equality_labels,
        } : null,
        canonical_result: smokeSnapshot.canonical_result ? {
          document_id_present: Boolean(smokeSnapshot.canonical_result.document_id),
          run_id_present: Boolean(smokeSnapshot.canonical_result.run_id),
          method_version: smokeSnapshot.canonical_result.method_version,
          primary_cell_id: smokeSnapshot.canonical_result.primary_cell_id,
          execution_cell_id: smokeSnapshot.canonical_result.execution_cell_id,
          capability_cell_ids: smokeSnapshot.canonical_result.capability_cell_ids,
          selected_table: (() => {
            const selected = smokeSnapshot.canonical_result.tables.find((candidate) => candidate.id === query.table_id);
            return selected ? { id: selected.id, rows_nonempty: selected.rows > 0, columns_nonempty: selected.columns > 0 } : null;
          })(),
          specific_indirect_count: smokeSnapshot.canonical_result.specific_indirect_count,
          interaction_effect_count: smokeSnapshot.canonical_result.interaction_effect_count,
          conditional_slope_count: smokeSnapshot.canonical_result.conditional_slope_count,
          conditional_probe_contracts: smokeSnapshot.canonical_result.conditional_probe_contracts,
          three_way_effect_count: smokeSnapshot.canonical_result.three_way_effect_count,
          three_way_conditional_effect_count: smokeSnapshot.canonical_result.three_way_conditional_effect_count,
          three_way_simple_slope_count: smokeSnapshot.canonical_result.three_way_simple_slope_count,
          three_way_probe_contracts: smokeSnapshot.canonical_result.three_way_probe_contracts,
          conditional_indirect_count: smokeSnapshot.canonical_result.conditional_indirect_count,
          moderated_mediation_index_count: smokeSnapshot.canonical_result.moderated_mediation_index_count,
          higher_order_stage_count: smokeSnapshot.canonical_result.higher_order_stage_count,
        } : null,
        legacy_result: smokeSnapshot.legacy_result,
      } : null,
      advanced_parameter_revision: query.include_advanced_parameter_revision === true && context.advancedParameterRevision ? {
        parameter_id_present: context.advancedParameterRevision.parameter_ids.length === 2
          && context.advancedParameterRevision.parameter_ids.every(Boolean),
        parameter_count: context.advancedParameterRevision.parameter_ids.length,
        before_digest_present: /^[a-f0-9]{64}$/u.test(context.advancedParameterRevision.before_model_document_sha256 ?? ""),
        after_digest_present: /^[a-f0-9]{64}$/u.test(context.advancedParameterRevision.after_model_document_sha256 ?? ""),
        equality_label: context.advancedParameterRevision.equality_label,
        stable_parameter_id: context.advancedParameterRevision.stable_parameter_id,
        changed_authority: context.advancedParameterRevision.changed_authority,
      } : null,
    };
  }
  if (kind === "cfa_compatibility_result") {
    assert(typeof query.table_id === "string" && SAFE_RESULT_ID.test(query.table_id), "cfa_compatibility_result query requires a safe table_id.");
    const table = await waitForSingleVisibleLocator(page, resultTableSelector(query.table_id), context.timeout);
    const rows = (await table.locator("tbody tr").allTextContents()).map(compact);
    const headers = (await table.locator("thead th").allTextContents()).map(compact);
    const smokeSnapshot = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
    const canonical = smokeSnapshot?.canonical_result ?? null;
    const includesEvery = (source, expected) => (expected ?? []).every((token) => source.some((value) => value.includes(token)));
    return {
      fixture: context.activeFixture?.fixture ?? null,
      selected_method: context.lastCalculation?.method ?? null,
      selected_inference: context.lastCalculation?.inference ?? null,
      route_expectations_passed: (query.route_contains ?? []).every((token) => (context.lastCalculation?.routeText ?? "").includes(token)),
      table_id: await table.getAttribute("data-canonical-table-id") ?? await table.getAttribute("data-result-table-id"),
      table_shape_passed: rows.length >= Number(query.minimum_rows ?? 1) && headers.length >= Number(query.minimum_columns ?? 1),
      header_expectations_passed: includesEvery(headers, query.header_contains),
      row_expectations_passed: includesEvery(rows, query.row_contains),
      canonical_document_id_present: Boolean(canonical?.document_id),
      canonical_method_version: canonical?.method_version ?? null,
      primary_cell_id: canonical?.primary_cell_id ?? null,
      execution_cell_id: canonical?.execution_cell_id ?? null,
      capability_cell_ids: canonical?.capability_cell_ids ?? [],
      model: smokeSnapshot?.model ? {
        ordinary_construct_count: smokeSnapshot.model.ordinary_construct_count,
        common_factor_count: smokeSnapshot.model.common_factor_count,
        structural_relation_count: smokeSnapshot.model.structural_relation_count,
        interaction_orders: smokeSnapshot.model.interaction_orders,
        higher_order_measurement_types: smokeSnapshot.model.higher_order_measurement_types,
      } : null,
    };
  }
  if (kind === "result_surface") {
    return page.evaluate(() => {
      const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
      const unique = (values) => [...new Set(values.map(clean).filter(Boolean))];
      const select = document.querySelector(".nd-results-nav .nd-run-select select");
      return {
        selected_result: select instanceof HTMLSelectElement ? clean(select.selectedOptions[0]?.textContent) : "",
        navigation: unique(Array.from(document.querySelectorAll(".nd-result-tree [role='treeitem']")).map((node) => node.textContent)),
        visible_headings: unique(Array.from(document.querySelectorAll(".nd-results-workspace h1, .nd-results-workspace h2, .nd-results-workspace h3")).map((node) => node.textContent)),
        visible_result_table_ids: unique(Array.from(document.querySelectorAll(".nd-results-workspace [data-result-table-id], .nd-results-workspace [data-canonical-table-id]")).map((node) => node.getAttribute("data-result-table-id") ?? node.getAttribute("data-canonical-table-id"))),
      };
    });
  }
  if (kind === "file") {
    const absolute = resolveRepoPath(query.path, "file observation");
    assert(inside(evidenceDir, absolute) || query.allow_existing_input === true, "File observation outside this evidence directory requires allow_existing_input=true.");
    if (!await exists(absolute)) return { exists: false, size_bytes: 0, sha256: null };
    const stat = await fs.stat(absolute);
    return { exists: stat.isFile(), size_bytes: stat.size, sha256: stat.isFile() ? await fileSha256(absolute) : null };
  }
  assert(typeof query.selector === "string" && query.selector, `${kind} query requires a CSS selector.`);
  const locator = page.locator(query.selector);
  if (kind === "text") return compact(await locator.first().textContent());
  if (kind === "attribute") { assert(typeof query.name === "string" && query.name, "attribute query requires name."); return locator.first().getAttribute(query.name); }
  if (kind === "count") return locator.count();
  if (kind === "visible") return locator.first().isVisible().catch(() => false);
  if (kind === "enabled") return locator.first().isEnabled().catch(() => false);
  if (kind === "input_value") return locator.first().inputValue();
  if (kind === "selected_text") return locator.first().locator("option:checked").textContent().then(compact);
  throw new Error(`Unsupported observation query kind: ${kind}`);
}

async function observeCompletedResultIdentity(page) {
  const snapshot = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
  const selectedValue = await page.locator(".nd-results-nav .nd-run-select select").inputValue();
  if (snapshot?.canonical_result) {
    const result = snapshot.canonical_result;
    assert(typeof result.document_id === "string" && SAFE_RESULT_ID.test(result.document_id), "Completed canonical result has no safe document identity.");
    assert(typeof result.run_id === "string" && result.run_id.trim(), "Completed canonical result has no run identity.");
    assert([result.document_id, `canonical:${result.document_id}`].includes(selectedValue), "Selected Results identity does not match the completed canonical document.");
    assert(typeof result.method_version === "string" && result.method_version.trim(), "Completed canonical result has no method version.");
    assert(Array.isArray(result.capability_cell_ids) && result.capability_cell_ids.length > 0
      && result.capability_cell_ids.every((cellId) => typeof cellId === "string" && cellId.trim()),
    "Completed canonical result has no exact capability-cell identity.");
    return {
      selected_value: selectedValue,
      result_identity: { type: "canonical_result_document_id", value: result.document_id },
      scientific_identity: {
        method_version: result.method_version,
        capability_cell_ids: [...result.capability_cell_ids].sort(),
      },
    };
  }
  const result = snapshot?.legacy_result;
  assert(result?.status === "completed" && typeof result.result_id === "string" && SAFE_RESULT_ID.test(result.result_id), "Completed result has neither a canonical document nor a safe schema-5 run identity.");
  assert(selectedValue === result.result_id, "Selected Results identity does not match the completed schema-5 run.");
  assert(typeof result.method_version === "string" && result.method_version.trim(), "Completed schema-5 result has no method version.");
  return {
    selected_value: selectedValue,
    result_identity: { type: "schema5_result_run_id", value: result.result_id },
    scientific_identity: { method_version: result.method_version, capability_cell_ids: [] },
  };
}

async function inspectSavedSupplementArchive(context, target, tableId, identity, timeout) {
  const receipt = await runJsonProcess(context.python, [
    ARCHIVE_IDENTITY_HELPER,
    "--archive", target,
    "--result-id", identity.result_identity.value,
    "--table-id", tableId,
  ], timeout);
  const expectedSchema = identity.result_identity.type === "canonical_result_document_id" ? 6 : 5;
  assert(receipt?.passed === true && receipt?.suite_id === "quickpls_v255_named_archive_identity_v1", "Saved result archive identity helper returned an invalid receipt.");
  assert(receipt.identity?.archive_schema === expectedSchema
    && receipt.identity?.result_id === identity.result_identity.value
    && receipt.identity?.status === "completed"
    && receipt.identity?.method_version === identity.scientific_identity.method_version
    && receipt.identity?.table_id === tableId,
  "Saved result archive does not contain the exact completed result/table identity.");
  if (expectedSchema === 6) {
    assert(deepEqual([...receipt.identity.capability_cell_ids].sort(), identity.scientific_identity.capability_cell_ids), "Saved canonical result capability cells differ from the live completed result.");
  }
  return receipt.identity;
}

async function executeStep(page, step, context) {
  assert(step && typeof step === "object" && typeof step.action === "string", "Every step requires an action.");
  const timeout = Number(step.timeout_ms ?? context.timeout);
  assert(Number.isInteger(timeout) && timeout >= 100 && timeout <= context.timeout, `Invalid step timeout: ${timeout}`);
  if (step.action === "goto_packaged") {
    const suffix = typeof step.path === "string" ? step.path : "/?quickpls_smoke=1";
    assert(suffix.startsWith("/") && !suffix.startsWith("//"), "goto_packaged path must be origin-relative.");
    await page.goto(`${PACKAGED_TAURI_ORIGIN}${suffix}`, { waitUntil: "domcontentloaded", timeout });
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
    await page.waitForFunction(() => (
      typeof window.__QUICKPLS_SMOKE__?.setView === "function"
      && typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture === "function"
      && typeof window.__QUICKPLS_SMOKE__?.loadNamedSemEvidenceFixture === "function"
    ), undefined, { timeout });
    return { action: step.action, url: page.url() };
  }
  if (step.action === "create_project") {
    assert(typeof step.name === "string" && step.name.trim(), "create_project requires a non-empty name.");
    const newProject = page.getByRole("button", { name: "New Project…", exact: true }).first();
    await newProject.waitFor({ state: "visible", timeout });
    assert(await newProject.isEnabled(), "The visible New Project command is disabled.");
    await newProject.click({ timeout });
    const dialog = page.getByRole("dialog", { name: "New Project", exact: true });
    await dialog.waitFor({ state: "visible", timeout });
    await dialog.getByLabel("Project name", { exact: true }).fill(step.name);
    await dialog.getByRole("button", { name: "Create", exact: true }).click();
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout });
    return { action: step.action, name: step.name };
  }
  if (step.action === "set_viewport") {
    assert(Number.isInteger(step.width) && Number.isInteger(step.height) && step.width >= 800 && step.height >= 600 && step.width <= 3840 && step.height <= 2160, "set_viewport dimensions are out of bounds.");
    await page.setViewportSize({ width: step.width, height: step.height });
    return { action: step.action, observed: await observe(page, { kind: "viewport" }, context.evidenceDir) };
  }
  if (step.action === "set_view") {
    assert(ALLOWED_VIEWS.has(step.view), `Unsupported QuickPLS view: ${step.view}`);
    const available = await page.evaluate((view) => {
      if (typeof window.__QUICKPLS_SMOKE__?.setView !== "function") return false;
      window.__QUICKPLS_SMOKE__.setView(view);
      return true;
    }, step.view);
    assert(available, "QuickPLS smoke view API is unavailable.");
    const surface = surfaceForView(step.view);
    await page.locator(`.nd-app[data-surface="${surface}"]`).waitFor({ state: "visible", timeout });
    return { action: step.action, requested_view: step.view };
  }
  if (step.action === "load_fixture") {
    assert(typeof step.fixture === "string" && step.fixture, "load_fixture requires fixture.");
    const available = await page.evaluate(async (fixture) => {
      if (typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture !== "function") return false;
      await window.__QUICKPLS_SMOKE__.loadDiagramFixture(fixture);
      return true;
    }, step.fixture);
    assert(available, "QuickPLS smoke fixture API is unavailable.");
    await page.locator('.nd-app[data-surface="model"]').waitFor({ state: "visible", timeout });
    await page.waitForTimeout(RENDERER_ERROR_SETTLE_MS);
    return { action: step.action, fixture: step.fixture };
  }
  if (step.action === "load_named_sem_fixture") {
    assert(typeof step.fixture === "string" && step.fixture, "load_named_sem_fixture requires fixture.");
    const observed = await page.evaluate(async ({ fixture, datasetPath }) => {
      if (typeof window.__QUICKPLS_SMOKE__?.loadNamedSemEvidenceFixture !== "function") return null;
      return window.__QUICKPLS_SMOKE__.loadNamedSemEvidenceFixture({ fixture, datasetPath });
    }, { fixture: step.fixture, datasetPath: context.namedSemDatasetPath });
    assert(observed
      && observed.fixture === step.fixture
      && typeof observed.modelId === "string"
      && observed.datasetResident === true
      && /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu.test(observed.datasetId ?? "")
      && typeof observed.datasetFingerprint === "string"
      && observed.datasetFingerprint.trim()
      && observed.datasetRows === 360
      && observed.datasetColumns === 20,
    "Named SEM fixture API is unavailable or did not return an exact native-resident dataset receipt.");
    context.activeFixture = observed;
    await page.locator('.nd-app[data-surface="model"]').waitFor({ state: "visible", timeout });
    return {
      action: step.action,
      source: {
        path: slash(path.relative(ROOT, context.namedSemDatasetPath)),
        sha256: context.namedSemDatasetSha256,
      },
      observed,
    };
  }
  if (step.action === "inspect_archive_identity") {
    const archive = resolveRepoPath(step.path, "inspect_archive_identity archive");
    assert(await exists(archive), `Archive is missing: ${archive}`);
    assert(typeof step.result_id === "string" && SAFE_RESULT_ID.test(step.result_id), "inspect_archive_identity requires a safe result_id.");
    assert(typeof step.table_id === "string" && SAFE_RESULT_ID.test(step.table_id), "inspect_archive_identity requires a safe table_id.");
    const receipt = await runJsonProcess(context.python, [
      ARCHIVE_IDENTITY_HELPER,
      "--archive", archive,
      "--result-id", step.result_id.replace(/^canonical:/u, ""),
      "--table-id", step.table_id,
    ], timeout);
    assert(receipt?.passed === true && receipt?.suite_id === "quickpls_v255_named_archive_identity_v1" && receipt.identity?.result_id === step.result_id.replace(/^canonical:/u, ""), "Archive identity helper returned an invalid receipt.");
    if (step.expected) assert(deepEqual(receipt.identity, step.expected), `Archive identity does not match the curated contract: ${JSON.stringify({ expected: step.expected, observed: receipt.identity })}`);
    context.archiveIdentity = receipt.identity;
    return { action: step.action, identity: receipt.identity };
  }
  if (step.action === "prepare_calculation_revision") {
    assert(new Set(["pls_algorithm", "pls_bootstrap", "cbsem"]).has(step.method),
      `Calculation-ready revision has an unsupported method: ${step.method}`);
    assert(step.inference === "point" || step.inference === "case_bootstrap",
      `Calculation-ready revision has invalid inference: ${step.inference}`);
    const target = path.join(context.evidenceDir, `${safeFileName(context.currentCaseId ?? context.activeFixture?.fixture ?? "named")}-calculation-ready.qpls`);
    assert(!await exists(target), `Calculation-ready revision target already exists: ${target}`);
    await page.getByRole("menubar", { name: "Application menu", exact: true }).getByRole("menuitem", { name: "Model", exact: true }).click();
    const command = page.getByRole("menuitem", { name: "Create Calculation-Ready Revision…", exact: true });
    if (await command.isVisible().catch(() => false) && await command.isEnabled().catch(() => false)) {
      await command.click();
    } else {
      const parameters = page.getByRole("menuitem", { name: "Advanced Parameter Table…", exact: true });
      await parameters.waitFor({ state: "visible", timeout });
      assert(await parameters.isEnabled(), "Advanced Parameter Table is unavailable for the calculation-ready revision fallback.");
      await parameters.click({ timeout });
      const parameterDialog = page.getByRole("dialog", { name: "Advanced Parameter Table", exact: true });
      await parameterDialog.waitFor({ state: "visible", timeout });
      const continueToCalculate = parameterDialog.getByRole("button", { name: "Continue to Calculate", exact: true });
      await continueToCalculate.waitFor({ state: "visible", timeout });
      assert(await continueToCalculate.isEnabled(), "Advanced Parameter Table cannot continue to the calculation-ready revision.");
      await continueToCalculate.click({ timeout });
    }
    const advanced = page.getByRole("dialog", { name: "Calculate Advanced Model", exact: true });
    await advanced.waitFor({ state: "visible", timeout });
    const estimator = advanced.locator("#nd-general-sem-estimator-recipe");
    await estimator.waitFor({ state: "visible", timeout });
    assert(await estimator.isEnabled(), "The fresh calculation-ready draft cannot select its requested estimator.");
    const estimatorId = step.method === "cbsem" ? "qpls.cbsem.v3" : "qpls.pls_sem.v3";
    const caseBootstrap = step.method === "pls_bootstrap" || (step.method === "cbsem" && step.inference === "case_bootstrap");
    await estimator.selectOption(estimatorId);
    const bootstrap = advanced.locator("#nd-general-sem-bootstrap");
    await bootstrap.waitFor({ state: "visible", timeout });
    assert(await bootstrap.isEnabled(), "The fresh calculation-ready draft cannot select its requested inference.");
    await bootstrap.setChecked(caseBootstrap);
    if (caseBootstrap) {
      const samples = advanced.locator("#nd-general-sem-bootstrap-samples");
      await samples.waitFor({ state: "visible", timeout });
      await samples.fill(String(step.bootstrap_samples ?? 500));
    }
    assert(await estimator.inputValue() === estimatorId, "The calculation-ready draft did not retain the requested estimator before activation.");
    assert(await bootstrap.isChecked() === caseBootstrap, "The calculation-ready draft did not retain the requested inference before activation.");
    const activate = advanced.getByRole("button", { name: "Save and activate project…", exact: true });
    await activate.waitFor({ state: "visible", timeout });
    const activationDeadline = Date.now() + timeout;
    while (Date.now() < activationDeadline && !await activate.isEnabled().catch(() => false)) await page.waitForTimeout(100);
    assert(await activate.isEnabled(), `Calculation-ready revision is blocked: ${compact(await advanced.textContent())}`);
    const helper = createDialogHelper({ python: context.python, mode: "save", target, allowedRoot: context.evidenceDir, extension: "qpls", candidatePid: context.candidatePid, candidatePath: context.candidatePath });
    try {
      const ready = await helper.ready;
      assert(ready?.passed === true && ready?.event === "ready", `Revision Save helper was not ready: ${JSON.stringify(ready)}`);
      await activate.click({ timeout });
      const completed = await helper.completed;
      assert(completed?.passed === true, `Calculation-ready revision Save failed: ${JSON.stringify(completed)}`);
    } finally { helper.stop(); }
    const activatedAuthority = advanced.getByText("Activated calculation authority", { exact: true });
    await activatedAuthority.waitFor({ state: "visible", timeout });
    const activatedEstimator = advanced.locator("#nd-general-sem-estimator-recipe");
    const activatedBootstrap = advanced.locator("#nd-general-sem-bootstrap");
    assert(await activatedEstimator.inputValue() === estimatorId, "The activated calculation authority changed the requested estimator.");
    assert(await activatedBootstrap.isChecked() === caseBootstrap, "The activated calculation authority changed the requested inference.");
    if (caseBootstrap) {
      const activatedSamples = advanced.locator("#nd-general-sem-bootstrap-samples");
      assert(await activatedSamples.inputValue() === String(step.bootstrap_samples ?? 500), "The activated calculation authority changed the requested bootstrap sample count.");
    }
    const calculateFromAuthority = advanced.locator(".nd-cbsem-v4-actions button.primary:not([disabled])").filter({ hasText: /^Calculate/u }).first();
    await calculateFromAuthority.waitFor({ state: "visible", timeout });
    assert(await calculateFromAuthority.isEnabled(), "The saved calculation authority did not become ready to calculate.");
    const close = advanced.getByRole("button", { name: "Close dialog", exact: true });
    if (await close.isVisible().catch(() => false)) await close.click();
    await advanced.waitFor({ state: "hidden", timeout });
    context.calculationRevision = { target, initialSha256: await fileSha256(target) };
    return {
      action: step.action,
      required: true,
      selected_method: step.method,
      selected_inference: step.inference,
      target: slash(path.relative(ROOT, target)),
      sha256: context.calculationRevision.initialSha256,
    };
  }
  if (step.action === "exercise_advanced_parameter_revision") {
    assert(context.calculationRevision?.target && await exists(context.calculationRevision.target), "Advanced Parameter Table requires an activated source archive.");
    const sourceTarget = context.calculationRevision.target;
    const sourceSha256 = await fileSha256(sourceTarget);
    await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout });
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
    await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot === "function", undefined, { timeout });
    await page.evaluate((archive) => window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: archive } })), sourceTarget);
    await waitForOpenedProjectPath(page, sourceTarget, timeout);
    assert(await fileSha256(sourceTarget) === sourceSha256, "Freshly reopening the Advanced Parameter source changed its archive bytes.");
    const before = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
    assert(/^[a-f0-9]{64}$/u.test(before?.model?.model_document_sha256 ?? ""), "Advanced Parameter Table requires a strict authority digest.");
    await page.getByRole("menubar", { name: "Application menu", exact: true }).getByRole("menuitem", { name: "Model", exact: true }).click();
    await page.getByRole("menuitem", { name: "Advanced Parameter Table…", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Advanced Parameter Table", exact: true });
    await dialog.waitFor({ state: "visible", timeout });
    const parameterRows = dialog.locator('tbody[aria-label="Parameters"] tr:not(.nd-sem-section-row)');
    const candidates = [];
    for (let index = 0; index < await parameterRows.count(); index += 1) {
      const row = parameterRows.nth(index);
      const objectKind = compact(await row.locator("td").nth(0).textContent());
      const specification = compact(await row.locator("td").nth(2).textContent());
      if (objectKind !== "Loading" || !specification.startsWith("Free;")) continue;
      const target = /^Free;\s*(.+?)\s+→\s+/u.exec(specification);
      if (!target?.[1]) continue;
      candidates.push({ index, compatibilityKey: target[1].trim() });
    }
    const compatibilityGroups = new Map();
    for (const candidate of candidates) {
      const group = compatibilityGroups.get(candidate.compatibilityKey) ?? [];
      group.push(candidate.index);
      compatibilityGroups.set(candidate.compatibilityKey, group);
    }
    const compatibleRowIndexes = [...compatibilityGroups.values()].find((indexes) => indexes.length >= 2)?.slice(0, 2) ?? [];
    assert(compatibleRowIndexes.length === 2, `The Advanced Parameter Table has no compatible pair of free loading parameters: ${JSON.stringify(candidates)}`);
    const parameterIds = [];
    for (const rowIndex of compatibleRowIndexes) {
      const freeParameterRow = parameterRows.nth(rowIndex);
      const edit = freeParameterRow.locator(".nd-sem-edit-button");
      await edit.waitFor({ state: "visible", timeout });
      await edit.click({ timeout });
      const editor = dialog.locator("#nd-sem-parameter-editor");
      await editor.waitFor({ state: "visible", timeout });
      const parameterId = compact(await editor.locator(".nd-sem-editor-heading code").textContent());
      assert(parameterId && !parameterIds.includes(parameterId), "The opened Advanced Parameter editor has no unique parameter identity.");
      parameterIds.push(parameterId);
      await editor.getByLabel("Equality label", { exact: true }).fill("V255Evidence");
      await editor.getByRole("button", { name: "Apply", exact: true }).click();
      await editor.waitFor({ state: "detached", timeout });
    }
    const deadline = Date.now() + timeout;
    let after = null;
    while (Date.now() < deadline) {
      after = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
      if (parameterIds.every((parameterId) => after?.model?.advanced_equalities?.some((item) => item.parameter_id === parameterId && item.equality_label === "V255Evidence"))
        && after?.model?.model_document_sha256 !== before.model.model_document_sha256) break;
      await page.waitForTimeout(100);
    }
    assert(parameterIds.every((parameterId) => after?.model?.advanced_equalities?.some((item) => item.parameter_id === parameterId && item.equality_label === "V255Evidence")), "The visible Advanced Parameter Table did not preserve both compatible parameter identities/equality labels.");
    assert(after.model.model_document_sha256 !== before.model.model_document_sha256, "The visible Advanced Parameter Table did not change authority.");
    const continueToCalculate = dialog.getByRole("button", { name: "Continue to Calculate", exact: true });
    await continueToCalculate.waitFor({ state: "visible", timeout });
    assert(await continueToCalculate.isEnabled(), "The edited Advanced Parameter Table cannot continue to its new calculation authority.");
    await continueToCalculate.click({ timeout });
    await dialog.waitFor({ state: "hidden", timeout });
    const advanced = page.getByRole("dialog", { name: "Calculate Advanced Model", exact: true });
    await advanced.waitFor({ state: "visible", timeout });
    const estimator = advanced.locator("#nd-general-sem-estimator-recipe");
    await estimator.waitFor({ state: "visible", timeout });
    assert(await estimator.isEnabled(), "The Advanced Parameter revision cannot select CB-SEM.");
    await estimator.selectOption("qpls.cbsem.v3");
    const bootstrap = advanced.locator("#nd-general-sem-bootstrap");
    await bootstrap.waitFor({ state: "visible", timeout });
    await bootstrap.setChecked(false);
    assert(await estimator.inputValue() === "qpls.cbsem.v3", "The Advanced Parameter revision did not retain CB-SEM before activation.");
    assert(!await bootstrap.isChecked(), "The Advanced Parameter revision did not retain point inference before activation.");
    const target = path.join(context.evidenceDir, `${safeFileName(context.currentCaseId)}-parameter-revision.qpls`);
    assert(target !== sourceTarget, "The Advanced Parameter revision must use a new archive path.");
    assert(!await exists(target), `Advanced Parameter revision target already exists: ${target}`);
    const activate = advanced.getByRole("button", { name: "Save and activate project…", exact: true });
    await activate.waitFor({ state: "visible", timeout });
    const activationDeadline = Date.now() + timeout;
    while (Date.now() < activationDeadline && !await activate.isEnabled().catch(() => false)) await page.waitForTimeout(100);
    assert(await activate.isEnabled(), `Advanced Parameter revision activation is blocked: ${compact(await advanced.textContent())}`);
    const helper = createDialogHelper({ python: context.python, mode: "save", target, allowedRoot: context.evidenceDir, extension: "qpls", candidatePid: context.candidatePid, candidatePath: context.candidatePath });
    try {
      const ready = await helper.ready;
      assert(ready?.passed === true && ready?.event === "ready", `Advanced Parameter revision Save helper was not ready: ${JSON.stringify(ready)}`);
      await activate.click({ timeout });
      const completed = await helper.completed;
      assert(completed?.passed === true, `Advanced Parameter revision Save failed: ${JSON.stringify(completed)}`);
    } finally { helper.stop(); }
    await advanced.getByText("Activated calculation authority", { exact: true }).waitFor({ state: "visible", timeout });
    assert(await estimator.inputValue() === "qpls.cbsem.v3" && !await bootstrap.isChecked(), "The activated Advanced Parameter authority changed its exact CB-SEM point selection.");
    const activated = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
    assert(parameterIds.every((parameterId) => activated?.model?.advanced_equalities?.some((item) => item.parameter_id === parameterId && item.equality_label === "V255Evidence")), "The activated Advanced Parameter authority lost one of the edited equality identities.");
    assert(activated?.model?.model_id === before.model.model_id, "The source-preserving Advanced Parameter revision changed the stable model identity.");
    assert(activated?.model?.model_document_sha256 === after.model.model_document_sha256, "The activated Advanced Parameter authority changed the edited model digest.");
    assert(await fileSha256(sourceTarget) === sourceSha256, "Creating the Advanced Parameter revision changed the source archive bytes.");
    const targetSha256 = await fileSha256(target);
    const close = advanced.getByRole("button", { name: "Close dialog", exact: true });
    if (await close.isVisible().catch(() => false)) await close.click();
    await advanced.waitFor({ state: "hidden", timeout });
    const observed = {
      parameter_ids: parameterIds,
      before_model_document_sha256: before.model.model_document_sha256,
      after_model_document_sha256: activated.model.model_document_sha256,
      before_model_id: before.model.model_id,
      after_model_id: activated.model.model_id,
      equality_label: "V255Evidence",
      stable_parameter_id: true,
      changed_authority: true,
      visible_dialog_workflow: true,
    };
    context.calculationRevision = { target, initialSha256: targetSha256, sourceTarget, sourceSha256 };
    context.advancedParameterRevision = observed;
    return {
      action: step.action,
      observed,
      source_target: slash(path.relative(ROOT, sourceTarget)),
      source_sha256: sourceSha256,
      target: slash(path.relative(ROOT, target)),
      target_sha256: targetSha256,
    };
  }
  if (step.action === "save_and_reopen_case_revision") {
    assert(context.calculationRevision?.target && await exists(context.calculationRevision.target), "No case calculation-ready revision is available to save and reopen.");
    const target = context.calculationRevision.target;
    const targetSha256 = await fileSha256(target);
    assert(targetSha256 === context.calculationRevision.initialSha256, "The activated Advanced Parameter revision changed before reopen.");
    if (context.calculationRevision.sourceTarget) {
      assert(context.calculationRevision.sourceTarget !== target, "The Advanced Parameter source and revision targets must differ.");
      assert(await fileSha256(context.calculationRevision.sourceTarget) === context.calculationRevision.sourceSha256, "The Advanced Parameter source archive changed before reopen.");
    }
    await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout });
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
    await page.evaluate((archive) => window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: archive } })), target);
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
    await waitForOpenedProjectPath(page, target, timeout);
    const snapshot = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.namedSemEvidenceSnapshot?.() ?? null);
    assert(snapshot?.model?.advanced_equality_labels?.includes("V255Evidence"), "The saved Advanced Parameter Table equality label did not survive reopen.");
    assert(context.advancedParameterRevision?.parameter_ids.every((parameterId) => snapshot?.model?.advanced_equalities?.some((item) => item.parameter_id === parameterId && item.equality_label === "V255Evidence")), "The stable Advanced Parameter identities did not survive reopen.");
    assert(snapshot?.model?.model_id === context.advancedParameterRevision?.after_model_id, "The reopened Advanced Parameter model identity differs from the activated revision.");
    assert(snapshot?.model?.model_document_sha256 === context.advancedParameterRevision?.after_model_document_sha256, "The reopened Advanced Parameter model digest differs from the activated revision.");
    assert(await fileSha256(target) === targetSha256, "Reopening the Advanced Parameter revision changed its archive bytes.");
    context.activeFixture = null;
    context.lastCalculation = null;
    context.archiveIdentity = null;
    return { action: step.action, target: slash(path.relative(ROOT, target)), sha256: targetSha256, equality_label_reopened: true, stable_parameter_id_reopened: true };
  }
  if (step.action === "run_calculation") {
    const labels = { pls_algorithm: "PLS-SEM Algorithm", pls_bootstrap: "PLS-SEM Bootstrapping", cbsem: "CB-SEM / CFA" };
    assert(Object.hasOwn(labels, step.method), `Unsupported run_calculation method: ${step.method}`);
    assert(typeof step.requires_requested_revision === "boolean", "run_calculation requires an explicit requested-revision contract.");
    const configureAndStart = async () => {
      await page.locator('[aria-label="Calculate…"]').first().click({ timeout });
      const dialog = page.getByRole("dialog", { name: "Calculate", exact: true });
      await dialog.waitFor({ state: "visible", timeout });
      const option = dialog.locator('#nd-calculation-method-list [role="option"]').filter({ hasText: new RegExp(`^${labels[step.method]}$`) });
      await option.click({ timeout });
      if (step.method === "pls_bootstrap") {
        const samples = dialog.locator("#nd-calculation-bootstrap-samples");
        if (await samples.isVisible().catch(() => false)) await samples.fill(String(step.bootstrap_samples ?? 500));
      }
      if (step.method === "cbsem") {
        const inference = dialog.locator("#nd-calculation-cbsem-inference");
        await inference.waitFor({ state: "visible", timeout });
        await inference.selectOption(step.inference === "case_bootstrap" ? "case_bootstrap" : "point");
        const samples = dialog.locator("#nd-calculation-cbsem-bootstrap-samples");
        if (step.inference === "case_bootstrap" && await samples.isVisible().catch(() => false)) await samples.fill(String(step.bootstrap_samples ?? 500));
      }
      let moderatedStage = null;
      if (step.moderated_stage) {
        const selection = dialog.locator("#nd-calculation-moderated-mediation-path");
        await selection.waitFor({ state: "visible", timeout });
        const optionRows = await selection.locator("option").evaluateAll((rows) => rows.map((row) => ({ value: row.value, text: String(row.textContent ?? "").replace(/\s+/g, " ").trim() })));
        const match = optionRows.find((row) => row.value && row.text.toLowerCase().includes(step.moderated_stage.replace("_", " ")));
        assert(match, `No ${step.moderated_stage} moderated-mediation path is available: ${JSON.stringify(optionRows)}`);
        await selection.selectOption(match.value);
        moderatedStage = step.moderated_stage;
      }
      const start = dialog.getByRole("button", { name: "Start calculation", exact: true });
      const startEnableDeadline = Date.now() + timeout;
      while (Date.now() < startEnableDeadline && !await start.isEnabled().catch(() => false)) {
        await page.waitForTimeout(100);
      }
      assert(await start.isEnabled().catch(() => false), `Calculation is blocked: ${compact(await dialog.textContent())}`);
      const routeText = compact(await dialog.textContent());
      for (const expected of step.route_contains ?? []) assert(routeText.includes(expected), `Calculation route omits ${expected}: ${routeText}`);
      context.lastCalculation = { method: step.method, inference: step.inference ?? (step.method === "pls_bootstrap" ? "case_bootstrap" : "point"), moderatedStage, routeText };
      await start.click({ timeout });
      await dialog.waitFor({ state: "hidden", timeout }).catch(() => undefined);
    };
    let requestedRevisionHelper = null;
    let requestedRevisionTarget = null;
    if (!step.requires_requested_revision) {
      const activeRevision = context.calculationRevision;
      assert(activeRevision?.target && await exists(activeRevision.target),
        "An already-activated calculation requires its exact calculation-ready archive.");
      await waitForOpenedProjectPath(page, activeRevision.target, timeout);
      assert(await fileSha256(activeRevision.target) === activeRevision.initialSha256,
        "The active calculation-ready archive changed before calculation started.");
    }
    if (step.requires_requested_revision) {
      assert(step.moderated_stage === "first_stage" || step.moderated_stage === "second_stage",
        "Requested calculation revisions are reserved for an exact moderated-mediation path selection.");
      requestedRevisionTarget = path.join(context.evidenceDir, `${safeFileName(context.currentCaseId)}-requested-calculation.qpls`);
      assert(!await exists(requestedRevisionTarget), `Requested-calculation revision target already exists: ${requestedRevisionTarget}`);
      requestedRevisionHelper = createDialogHelper({ python: context.python, mode: "save", target: requestedRevisionTarget, allowedRoot: context.evidenceDir, extension: "qpls", candidatePid: context.candidatePid, candidatePath: context.candidatePath });
      const ready = await requestedRevisionHelper.ready;
      assert(ready?.passed === true && ready?.event === "ready", `Requested-calculation Save helper was not ready: ${JSON.stringify(ready)}`);
    }
    const advanced = page.getByRole("dialog", { name: "Calculate Advanced Model", exact: true });
    const advancedProgress = advanced.locator(".nd-cbsem-v4-monitor").filter({ visible: true });
    const activatedAuthority = advanced.getByText("Activated calculation authority", { exact: true }).filter({ visible: true });
    const exactCompatibility = advanced.locator("#nd-cbsem-compatibility-calculation").filter({ visible: true });
    const resultsWorkspace = page.locator(".nd-results-workspace").filter({ visible: true });
    const recoveryClickTimeout = Math.min(timeout, 1_000);
    const calculationAdvancedAutomatically = async () => (
      await advancedProgress.isVisible().catch(() => false)
      || await resultsWorkspace.isVisible().catch(() => false)
    );
    try {
      await configureAndStart();
      if (requestedRevisionHelper) {
        await advanced.waitFor({ state: "visible", timeout });
        const activate = advanced.getByRole("button", { name: "Save and activate project…", exact: true });
        const activationDeadline = Date.now() + timeout;
        let activationStarted = false;
        while (Date.now() < activationDeadline) {
          // The product's one-click coordinator may begin the same exact save
          // before Playwright reaches the recovery control. Existing target
          // bytes prove that activation is already underway; never double-click.
          if (await exists(requestedRevisionTarget)) {
            activationStarted = true;
            break;
          }
          if (await activate.isVisible().catch(() => false)
            && await activate.isEnabled().catch(() => false)) {
            const activationClicked = await activate.click({ timeout: recoveryClickTimeout })
              .then(() => true)
              .catch(() => false);
            if (activationClicked || await exists(requestedRevisionTarget)) {
              activationStarted = true;
              break;
            }
          }
          await page.waitForTimeout(100);
        }
        assert(activationStarted, `Requested-calculation revision never exposed an enabled activation control: ${compact(await advanced.textContent())}`);
        const completed = await requestedRevisionHelper.completed;
        assert(completed?.passed === true, `Requested-calculation revision Save failed: ${JSON.stringify(completed)}`);
        const requestedRevisionSha256 = String(completed.file?.sha256 ?? "").toLocaleLowerCase("en-US");
        assert(/^[0-9a-f]{64}$/u.test(requestedRevisionSha256),
          `Requested-calculation Save did not report an exact activation SHA-256: ${JSON.stringify(completed)}`);
        context.calculationRevision = { target: requestedRevisionTarget, initialSha256: requestedRevisionSha256 };
        await waitForOpenedProjectPath(page, requestedRevisionTarget, timeout);
        const calculate = advanced.getByRole("button", { name: "Calculate moderated-mediation bootstrap", exact: true });
        const calculationStartDeadline = Date.now() + timeout;
        let calculationStarted = false;
        while (Date.now() < calculationStartDeadline) {
          // The calculation-presentation coordinator may advance immediately
          // after activation, removing the transient authority/setup view.
          if (await calculationAdvancedAutomatically()) {
            calculationStarted = true;
            break;
          }
          const activatedAndCalculable = await activatedAuthority.isVisible().catch(() => false)
            && await calculate.isVisible().catch(() => false)
            && await calculate.isEnabled().catch(() => false);
          if (activatedAndCalculable) {
            const requestedRevisionLiveSha256 = await fileSha256(requestedRevisionTarget);
            if (await calculationAdvancedAutomatically()) {
              calculationStarted = true;
              break;
            }
            const stillActivatedAndCalculable = await activatedAuthority.isVisible().catch(() => false)
              && await calculate.isVisible().catch(() => false)
              && await calculate.isEnabled().catch(() => false);
            if (!stillActivatedAndCalculable) {
              await page.waitForTimeout(100);
              continue;
            }
            assert(requestedRevisionLiveSha256 === requestedRevisionSha256,
              "The activated requested-calculation archive changed before calculation started.");
            const calculationClicked = await calculate.click({ timeout: recoveryClickTimeout })
              .then(() => true)
              .catch(() => false);
            if (calculationClicked || await calculationAdvancedAutomatically()) {
              calculationStarted = true;
              break;
            }
          }
          await page.waitForTimeout(100);
        }
        assert(calculationStarted, `The activated moderated-mediation authority never exposed an enabled Calculate action: ${compact(await advanced.textContent())}`);
      }
    } finally {
      requestedRevisionHelper?.stop();
    }
    const transitionDeadline = Date.now() + (step.requires_requested_revision ? timeout : Math.min(timeout, 30_000));
    let transitionReady = false;
    while (Date.now() < transitionDeadline) {
      transitionReady = await resultsWorkspace.isVisible().catch(() => false)
        || await exactCompatibility.isVisible().catch(() => false)
        || await advancedProgress.isVisible().catch(() => false)
        || await activatedAuthority.isVisible().catch(() => false);
      if (transitionReady) break;
      await page.waitForTimeout(100);
    }
    assert(transitionReady, `The calculation authority neither activated nor continued into calculation progress: ${compact(await advanced.textContent().catch(() => ""))}`);
    const deadline = Date.now() + Number(step.completion_timeout_ms ?? 300_000);
    while (Date.now() < deadline) {
      if (await resultsWorkspace.isVisible().catch(() => false)) return { action: step.action, ...context.lastCalculation, terminal: "results" };
      const exactPublicationFailure = advanced.locator("#nd-cbsem-compatibility-calculation .nd-cbsem-v4-archive .nd-cbsem-v4-failure").filter({ visible: true });
      if (await exactPublicationFailure.isVisible().catch(() => false)) {
        throw new Error(`Exact CFA result publication failed: ${compact(await exactPublicationFailure.textContent())}`);
      }
      const calculationFailure = advanced.locator(".nd-cbsem-v4-failure").filter({ visible: true });
      if (await calculationFailure.isVisible().catch(() => false)) {
        throw new Error(`Calculation publication failed: ${compact(await calculationFailure.textContent())}`);
      }
      const state = compact(await advanced.locator(".nd-cbsem-v4-state").filter({ visible: true }).textContent().catch(() => "")).toLowerCase();
      if (["failed", "cancelled"].includes(state)) throw new Error(`Calculation ended ${state}: ${compact(await advancedProgress.textContent())}`);
      await page.waitForTimeout(250);
    }
    throw new Error(`Calculation did not open Results within the bounded timeout: ${context.lastCalculation?.routeText ?? ""}`);
  }
  if (step.action === "save_result_archive_supplement") {
    assert(context.candidateName === "portable", "Fresh result archive supplements may only be captured from the portable candidate.");
    assert(ARCHIVE_SUPPLEMENT_PUBLIC_KINDS.has(step.public_kind), "Result archive supplement public method kind is unsupported.");
    assert(typeof step.table_id === "string" && SAFE_RESULT_ID.test(step.table_id), "Result archive supplement requires a safe table ID.");
    assert(typeof step.method_version === "string" && step.method_version.trim(), "Result archive supplement requires an exact method version.");
    assert(Array.isArray(step.capability_cell_ids), "Result archive supplement requires exact capability-cell identities.");
    assert(context.calculationRevision?.target && await exists(context.calculationRevision.target), "No existing calculation-ready archive is available for the completed result supplement.");
    assert(!context.resultArchiveSupplement, "A named case may emit at most one result archive supplement.");
    const target = context.calculationRevision.target;
    assert(inside(context.evidenceDir, target), "Result archive supplement target escapes this named-case evidence directory.");
    await waitForOpenedProjectPath(page, target, timeout);
    const liveIdentity = await observeCompletedResultIdentity(page);
    assert(liveIdentity.scientific_identity.method_version === step.method_version, "Completed result method version differs from the manifest route.");
    assert(deepEqual(liveIdentity.scientific_identity.capability_cell_ids, [...step.capability_cell_ids].sort()), "Completed result capability cells differ from the manifest route.");
    const beforeSha256 = await fileSha256(target);
    let afterSha256 = beforeSha256;
    let persistenceMode = "already_persisted";
    if (beforeSha256 === context.calculationRevision.initialSha256) {
      persistenceMode = "explicit_control_s";
      await page.keyboard.press("Control+s");
      const saveDeadline = Date.now() + timeout;
      while (Date.now() < saveDeadline) {
        await page.waitForTimeout(100);
        try { afterSha256 = await fileSha256(target); } catch { continue; }
        if (afterSha256 !== beforeSha256) break;
      }
      assert(afterSha256 !== beforeSha256, "The completed result was neither autosaved nor persisted by the explicit save command.");
    }
    const archiveIdentity = await inspectSavedSupplementArchive(context, target, step.table_id, liveIdentity, timeout);

    await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout });
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
    await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.setView === "function", undefined, { timeout });
    await page.evaluate((archive) => window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: archive } })), target);
    await waitForOpenedProjectPath(page, target, timeout);
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
    const resultSelect = page.locator(".nd-results-nav .nd-run-select select");
    await resultSelect.waitFor({ state: "visible", timeout });
    await resultSelect.locator(`option[value="${liveIdentity.selected_value}"]`).waitFor({ state: "attached", timeout });
    await resultSelect.selectOption(liveIdentity.selected_value);
    const visibleTable = await waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), 0, { allowAbsent: true });
    if (!visibleTable) {
      const targetItem = await waitForSingleVisibleLocator(page, resultTreeItemSelector(step.table_id), timeout);
      await targetItem.click({ timeout });
      await waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), timeout);
    }
    const reopenedIdentity = await observeCompletedResultIdentity(page);
    assert(deepEqual(reopenedIdentity.result_identity, liveIdentity.result_identity)
      && deepEqual(reopenedIdentity.scientific_identity, liveIdentity.scientific_identity),
    "Fresh reopen did not preserve the exact completed result scientific identity.");
    assert(await fileSha256(target) === afterSha256, "Fresh reopen changed the saved result archive bytes.");
    const archiveStat = await fs.stat(target);
    context.resultArchiveSupplement = {
      case_id: context.currentCaseId,
      public_kind: step.public_kind,
      status: "passed",
      archive: { path: slash(path.relative(ROOT, target)), sha256: afterSha256, size_bytes: archiveStat.size },
      result_identity: liveIdentity.result_identity,
      scientific_identity: liveIdentity.scientific_identity,
      candidate: {
        name: context.candidateName,
        pid: context.candidatePid,
        path: context.candidatePath,
        sha256: context.candidateSha256,
      },
      manifest: { path: context.manifestPath, sha256: context.manifestSha256 },
    };
    return {
      action: step.action,
      public_kind: step.public_kind,
      persistence_mode: persistenceMode,
      archive: context.resultArchiveSupplement.archive,
      result_identity: liveIdentity.result_identity,
      archive_identity: archiveIdentity,
      before_sha256: beforeSha256,
      saved_sha256: afterSha256,
      fresh_reopen_verified: true,
    };
  }
  if (step.action === "open_archive") {
    const archive = resolveRepoPath(step.path, "open_archive");
    assert(await exists(archive), `Archive is missing: ${archive}`);
    const archiveSha256 = await fileSha256(archive);
    if (step.expected_sha256 !== undefined) {
      assert(/^[a-f0-9]{64}$/u.test(step.expected_sha256), "open_archive expected_sha256 must be a lowercase SHA-256.");
      assert(archiveSha256 === step.expected_sha256, `Archive SHA-256 differs from the curated route: ${step.path}`);
    }
    await page.evaluate((target) => window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } })), archive);
    await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout });
    await waitForOpenedProjectPath(page, archive, timeout);
    context.activeFixture = null;
    context.lastCalculation = null;
    context.archiveIdentity = null;
    return { action: step.action, path: slash(path.relative(ROOT, archive)), sha256: archiveSha256 };
  }
  if (step.action === "select_result") {
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
    const select = page.locator(".nd-results-nav .nd-run-select select");
    await select.waitFor({ state: "visible", timeout });
    assert(typeof step.value === "string" || typeof step.label === "string", "select_result requires value or label.");
    await select.selectOption(typeof step.value === "string" ? { value: step.value } : { label: step.label });
    await page.locator(".nd-results-workspace").waitFor({ state: "visible", timeout });
    return { action: step.action, selected_value: await select.inputValue(), selected_text: compact(await select.locator("option:checked").textContent()) };
  }
  if (step.action === "select_result_table") {
    assert(typeof step.table_id === "string" && SAFE_RESULT_ID.test(step.table_id), "select_result_table requires a safe table_id.");
    const visibleTable = await waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), 0, { allowAbsent: true });
    if (visibleTable) return { action: step.action, table_id: step.table_id, already_visible: true };
    const target = await waitForSingleVisibleLocator(page, resultTreeItemSelector(step.table_id), timeout);
    await target.click({ timeout });
    await waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), timeout);
    return { action: step.action, table_id: step.table_id };
  }
  if (step.action === "wait_for") {
    assert(typeof step.selector === "string" && step.selector && ALLOWED_STATES.has(step.state ?? "visible"), "wait_for selector/state is invalid.");
    await page.locator(step.selector).first().waitFor({ state: step.state ?? "visible", timeout });
    return { action: step.action, selector: step.selector, state: step.state ?? "visible" };
  }
  if (["click", "double_click", "fill", "select_option", "press"].includes(step.action)) {
    assert(typeof step.selector === "string" && step.selector, `${step.action} requires selector.`);
    if (step.action === "press" && step.selector === "body") {
      assert(typeof step.key === "string" && step.key, "press requires key.");
      await page.keyboard.press(step.key);
      return { action: step.action, selector: step.selector };
    }
    const locator = page.locator(step.selector).first();
    await locator.waitFor({ state: "visible", timeout });
    if (step.action === "click") await locator.click({ timeout });
    else if (step.action === "double_click") await locator.dblclick({ timeout });
    else if (step.action === "fill") { assert(typeof step.value === "string", "fill requires string value."); await locator.fill(step.value, { timeout }); }
    else if (step.action === "select_option") {
      assert(typeof step.value === "string" || typeof step.label === "string", "select_option requires value or label.");
      await locator.selectOption(typeof step.value === "string" ? { value: step.value } : { label: step.label }, { timeout });
    } else {
      assert(typeof step.key === "string" && step.key, "press requires key.");
      const enabledDeadline = Date.now() + timeout;
      while (Date.now() < enabledDeadline && !await locator.isEnabled().catch(() => false)) await page.waitForTimeout(50);
      assert(await locator.isEnabled().catch(() => false), `Keyboard target did not become enabled: ${step.selector}`);
      await locator.focus({ timeout });
      const focusVerified = await locator.evaluate((element) => document.activeElement === element).catch(() => false);
      assert(focusVerified, `Keyboard target did not retain focus: ${step.selector}`);
      await locator.press(step.key, { timeout });
    }
    return { action: step.action, selector: step.selector };
  }
  if (step.action === "native_file_dialog") {
    assert(new Set(["open", "save"]).has(step.mode), "native_file_dialog mode must be open or save.");
    assert(typeof step.trigger_selector === "string" && step.trigger_selector, "native_file_dialog requires trigger_selector.");
    assert(typeof step.extension === "string" && /^[a-z0-9]{1,8}$/i.test(step.extension), "native_file_dialog extension is invalid.");
    const target = resolveRepoPath(step.target, "native_file_dialog target");
    const allowedRoot = step.mode === "save" ? context.evidenceDir : ROOT;
    assert(inside(allowedRoot, target), "native_file_dialog target escapes its allowed root.");
    assert(step.mode === "open" ? await exists(target) : !await exists(target), `native_file_dialog ${step.mode} target has an invalid precondition.`);
    const helper = createDialogHelper({ python: context.python, mode: step.mode, target, allowedRoot, extension: step.extension, candidatePid: context.candidatePid, candidatePath: context.candidatePath });
    try {
      const ready = await helper.ready;
      assert(ready?.passed === true && ready?.event === "ready", `Native file helper was not ready: ${JSON.stringify(ready)}`);
      await page.locator(step.trigger_selector).first().click({ timeout });
      const completed = await helper.completed;
      assert(completed?.passed === true && completed?.event === "complete", `Native file dialog failed: ${JSON.stringify(completed)}`);
      return { action: step.action, mode: step.mode, target: slash(path.relative(ROOT, target)), helper_phase: completed.phase ?? null };
    } finally { helper.stop(); }
  }
  if (step.action === "assert") {
    const observed = await observe(page, step.query, context.evidenceDir, context);
    assert(Object.hasOwn(step, "expected") && deepEqual(observed, step.expected), `Step assertion failed: ${JSON.stringify({ expected: step.expected, observed })}`);
    return { action: step.action, observed };
  }
  throw new Error(`Unsupported named-case step action: ${step.action}`);
}

async function runCase(page, entry, ordinal, context) {
  const record = { id: entry.id, status: "failed", candidate: entry.candidate, operation: entry.operation, steps: [], assertion: null, screenshot: null };
  try {
    context.currentCaseId = entry.id;
    context.activeFixture = null;
    context.lastCalculation = null;
    context.archiveIdentity = null;
    context.advancedParameterRevision = null;
    context.calculationRevision = null;
    context.resultArchiveSupplement = null;
    for (const step of entry.steps) record.steps.push(await executeStep(page, step, context));
    const observed = await observe(page, entry.assertion.query, context.evidenceDir, context);
    assert(deepEqual(observed, entry.assertion.expected), `${entry.id} final assertion failed: ${JSON.stringify({ expected: entry.assertion.expected, observed })}`);
    const screenshotFile = path.join(context.screens, `${String(ordinal).padStart(2, "0")}-${safeFileName(entry.id)}.png`);
    if (entry.screenshot?.selector) {
      const screenshotTarget = await waitForSingleVisibleLocator(page, entry.screenshot.selector, context.timeout);
      await screenshotTarget.screenshot({ path: screenshotFile, animations: "disabled" });
    } else {
      await page.screenshot({ path: screenshotFile, animations: "disabled", fullPage: entry.screenshot?.full_page === true });
    }
    const screenshotHash = await fileSha256(screenshotFile);
    record.assertion = { id: entry.assertion.id, expected: entry.assertion.expected, observed, passed: true };
    record.screenshot = { path: screenshotFile, sha256: screenshotHash };
    if (context.resultArchiveSupplement) record.result_archive_supplement = context.resultArchiveSupplement;
    record.status = "passed";
    record.observation = {
      schema_version: 1,
      case_id: entry.id,
      operation: entry.operation,
      assertion: { id: entry.assertion.id, passed: true, expected: entry.assertion.expected, observed },
      screenshot: { path: screenshotFile, sha256: screenshotHash },
    };
  } catch (error) {
    record.failure = error instanceof Error ? error.message : String(error);
    const failureFile = path.join(context.screens, `${String(ordinal).padStart(2, "0")}-${safeFileName(entry.id)}-failure.png`);
    await page.screenshot({ path: failureFile, animations: "disabled" }).catch(() => undefined);
    if (await exists(failureFile)) record.failure_screenshot = { path: failureFile, sha256: await fileSha256(failureFile) };
  }
  return record;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  assertLoopback(args.endpoint);
  assert(inside(RESULTS, args.evidenceDir) && args.evidenceDir !== RESULTS, "--evidence-dir must be a new child of validation/results.");
  assert(!await exists(args.evidenceDir), `Refusing to reuse evidence directory: ${args.evidenceDir}`);
  assert(inside(ROOT, args.manifest) && inside(ROOT, args.index), "Manifest and index must remain in the repository.");
  assert(await exists(args.manifest) && await exists(args.index) && await exists(NAMED_SEM_DATASET) && await exists(args.python)
    && await exists(FILE_DIALOG_HELPER) && await exists(ARCHIVE_IDENTITY_HELPER) && await exists(args.candidatePath),
  "Driver input, Python, archive identity helper, or native dialog helper is missing.");
  const candidateStat = await fs.stat(args.candidatePath);
  assert(candidateStat.isFile() && candidateStat.size > 0, "--candidate-path must be an existing non-empty executable file.");
  const candidateSha256 = await fileSha256(args.candidatePath);
  await fs.mkdir(args.evidenceDir, { recursive: false });
  const screens = path.join(args.evidenceDir, "screens");
  await fs.mkdir(screens, { recursive: false });
  const reportPath = path.join(args.evidenceDir, "v255_named_case_driver.json");
  const manifest = await readJson(args.manifest);
  const index = await readJson(args.index);
  const manifestPath = slash(path.relative(ROOT, args.manifest));
  const manifestSha256 = await fileSha256(args.manifest);
  const namedSemDatasetPath = path.resolve(NAMED_SEM_DATASET);
  const namedSemDatasetSha256 = await fileSha256(namedSemDatasetPath);
  const report = {
    schema_version: 1,
    suite_id: SUITE_ID,
    target_release: TARGET_RELEASE,
    status: "failed",
    passed: false,
    candidate: args.candidateName,
    candidate_process: {
      pid: args.candidatePid,
      executable: args.candidatePath,
      executable_sha256: candidateSha256,
    },
    sources: {
      manifest: manifestPath,
      manifest_sha256: manifestSha256,
      index: slash(path.relative(ROOT, args.index)),
      index_sha256: await fileSha256(args.index),
      named_sem_dataset: slash(path.relative(ROOT, namedSemDatasetPath)),
      named_sem_dataset_sha256: namedSemDatasetSha256,
    },
    serial: true,
    maximum_concurrent_cases: 1,
    console_errors: [],
    named_evidence_observations: [],
    result_archive_supplements: [],
    cases: [],
    offline: null,
    failures: [],
    process_safety: {
      wrapper_owns_candidate_process: true,
      candidate_pid_bound: true,
      candidate_executable_bound: true,
      driver_launches_candidate_processes: false,
      driver_terminates_candidate_processes: false,
      driver_closes_browser_page_or_context: false,
    },
  };
  let offline;
  let rendererErrors;
  try {
    const { selectedCases } = validateManifest(manifest, index, args.candidateName);
    const connection = await connectToSingleQuickPlsPage({ chromium, endpoint: args.endpoint });
    const page = connection.page;
    rendererErrors = observeRendererErrors(page);
    offline = observeFunctionalOfflineRequests(page);
    const context = {
      timeout: args.timeout,
      evidenceDir: args.evidenceDir,
      screens,
      python: args.python,
      candidateName: args.candidateName,
      candidatePid: args.candidatePid,
      candidatePath: args.candidatePath,
      candidateSha256,
      manifestPath,
      manifestSha256,
      namedSemDatasetPath,
      namedSemDatasetSha256,
    };
    for (let ordinal = 1; ordinal <= selectedCases.length; ordinal += 1) {
      if (page.isClosed()) {
        for (const remaining of selectedCases.slice(ordinal - 1)) {
          report.cases.push({
            id: remaining.id,
            status: "failed",
            candidate: remaining.candidate,
            operation: remaining.operation,
            steps: [],
            assertion: null,
            screenshot: null,
            failure: "The candidate renderer page closed before this serial named case started.",
          });
        }
        break;
      }
      report.cases.push(await runCase(page, selectedCases[ordinal - 1], ordinal, context));
    }
    report.offline = offline.summary();
    const screenshotHashes = report.cases.filter((entry) => entry.status === "passed").map((entry) => entry.screenshot.sha256);
    assert(report.cases.length === selectedCases.length && report.cases.every((entry) => entry.status === "passed"), "One or more named cases failed.");
    assert(new Set(screenshotHashes).size === screenshotHashes.length, "Two named cases produced identical screenshot bytes; case-specific screenshots must be unique.");
    assert(report.offline.passed, `Named-case driver observed an external request: ${JSON.stringify(report.offline)}`);
    report.named_evidence_observations = report.cases.map((entry) => entry.observation);
    report.result_archive_supplements = report.cases.flatMap((entry, index) => entry.result_archive_supplement ? [{
      ...entry.result_archive_supplement,
      source_report_json_pointer: `#/cases/${index}/result_archive_supplement`,
    }] : []);
    const expectedSupplementCases = selectedCases.filter((entry) => entry.route?.archive_supplement_public_kind !== undefined);
    assert(report.result_archive_supplements.length === expectedSupplementCases.length,
      `Expected ${expectedSupplementCases.length} result archive supplements, observed ${report.result_archive_supplements.length}.`);
    assert(deepEqual(report.result_archive_supplements.map((entry) => entry.case_id).sort(), expectedSupplementCases.map((entry) => entry.id).sort()),
      "Result archive supplements do not exactly cover the manifest opt-in cases.");
    for (const supplement of report.result_archive_supplements) {
      const archive = resolveRepoPath(supplement.archive.path, "result archive supplement");
      const stat = await fs.stat(archive);
      assert(stat.isFile() && stat.size === supplement.archive.size_bytes && await fileSha256(archive) === supplement.archive.sha256,
        `Result archive supplement changed before report finalization: ${supplement.case_id}`);
    }
    assert(await fileSha256(args.manifest) === manifestSha256, "Named-case manifest changed during collection.");
    assert(await fileSha256(namedSemDatasetPath) === namedSemDatasetSha256, "Named SEM dataset changed during collection.");
    assert(await fileSha256(args.candidatePath) === candidateSha256, "Packaged candidate bytes changed during named-case collection.");
    report.status = "passed";
    report.passed = true;
  } catch (error) {
    report.failures.push(error instanceof Error ? error.message : String(error));
  } finally {
    if (rendererErrors) {
      try {
        await rendererErrors.settle();
      } catch (error) {
        report.failures.push(`Renderer error observation did not settle: ${error instanceof Error ? error.message : String(error)}`);
      }
      report.console_errors = [...rendererErrors.errors];
      rendererErrors.stop();
    }
    if (report.console_errors.length > 0) {
      report.failures.push(`Renderer errors were observed: ${JSON.stringify(report.console_errors)}`);
    }
    if (report.failures.length > 0 || report.console_errors.length > 0) {
      report.status = "failed";
      report.passed = false;
    }
    offline?.stop();
    report.completed_at = new Date().toISOString();
    await writeJsonNew(reportPath, report);
  }
  return { report, reportPath };
}

let outcome;
try {
  outcome = await main();
} catch (error) {
  await new Promise((resolve) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`, resolve);
  });
  process.exit(1);
}
await new Promise((resolve, reject) => {
  process.stdout.write(`${JSON.stringify({ passed: outcome.report.passed, cases: outcome.report.cases.length, report: outcome.reportPath }, null, 2)}\n`, (error) => {
    if (error) reject(error);
    else resolve();
  });
});
// This attach-only driver never owns the candidate. Exit the local CDP client
// after the report and stdout are flushed; the PowerShell supervisor retains
// sole authority over the exact QuickPLS process tree.
process.exit(outcome.report.passed ? 0 : 1);
