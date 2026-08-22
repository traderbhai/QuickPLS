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
 *   --named-supplement-report validation/results/<portable-named-case-report>.json
 *   --timeout-ms 60000
 *
 * Inventory-v2 coverage routes may bind a static inventory archive, the exact
 * posthoc execute/reopen supplement, or one of seven portable named-result
 * supplements. Missing or mismatched dynamic provenance fails only its outer
 * method receipt; no result family is inferred from another route.
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
const INVENTORY_SCHEMA = "quickpls.v255.reusable_archive_inventory.v2";
const SUITE_ID = "quickpls_v255_frozen_archive_reopen_crawler_v1";
const NAMED_SUPPLEMENT_SUITE_ID = "quickpls_v255_named_case_driver_v1";
const OBSERVATION_SOURCES = new Set([
  "navigation",
  "visible_headings",
  "visible_result_table_ids",
  "table_titles",
  "table_headers",
  "table_rows",
  "chart_titles",
]);
const SAFE_DOM_ID = /^[a-zA-Z0-9:._-]+$/u;

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

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

async function runStage(methodKind, stage, action) {
  try {
    return await action();
  } catch (error) {
    const stagedError = new Error(`[${methodKind}][${stage}] ${errorMessage(error)}`);
    stagedError.quickplsStage = stage;
    stagedError.cause = error;
    throw stagedError;
  }
}

function errorStage(error) {
  return error && typeof error === "object" && typeof error.quickplsStage === "string"
    ? error.quickplsStage
    : null;
}

async function autosaveSiblingNames(archive) {
  const prefix = `${path.basename(archive)}.autosave.tmp-`;
  return (await fs.readdir(path.dirname(archive), { withFileTypes: true }))
    .filter((entry) => entry.name.startsWith(prefix))
    .map((entry) => entry.name)
    .sort();
}

async function namedObservationFromReceipt({ staging, receiptArtifact, caseId, observed }) {
  const receiptPath = path.join(staging, ...receiptArtifact.member.split("/"));
  const receipt = await readJson(receiptPath);
  assert(receipt.status === "verified_current_ui_capture", `${caseId} source receipt is not a verified current UI capture.`);
  assert(receipt.schema_version === 2 && Array.isArray(receipt.evidence) && receipt.evidence.length > 0, `${caseId} source receipt has no schema-2 capture evidence.`);
  const screenshotPath = path.join(staging, ...receipt.evidence[0].screenshot.member.split("/"));
  assert(inside(staging, screenshotPath) && await exists(screenshotPath), `${caseId} screenshot is missing from frozen staging.`);
  const screenshotHash = await fileSha256(screenshotPath);
  assert(screenshotHash === receipt.evidence[0].screenshot.sha256, `${caseId} screenshot does not match its frozen receipt.`);
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
  const allowed = new Set([
    "endpoint",
    "staging-dir",
    "inventory",
    "posthoc-supplement",
    "named-supplement-report",
    "timeout-ms",
  ]);
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

function deepEqual(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function compact(value) {
  return String(value ?? "").replace(/\s+/gu, " ").trim();
}

function validateResultIdentity(identity, kind) {
  assert(identity && typeof identity === "object", `${kind} has no declared result identity.`);
  assert(["canonical_result_document_id", "schema5_result_run_id"].includes(identity.type), `${kind} has unsupported identity type '${identity.type}'.`);
  assert(typeof identity.value === "string" && identity.value.trim(), `${kind} has an empty declared identity.`);
  if (identity.type === "canonical_result_document_id") {
    assert(identity.value.startsWith("result_"), `${kind} canonical result-document ID must start with 'result_'.`);
  }
}

function validateDeclaredArchiveSha256(row) {
  if (row.archive_sha256 === undefined) return;
  assert(typeof row.archive_sha256 === "string" && /^[0-9a-f]{64}$/u.test(row.archive_sha256), `${row.public_kind} archive_sha256 must be an exact lowercase SHA-256 digest when supplied.`);
}

function validateCaptureDeclaration(route, capture) {
  const label = `${route.public_kind}/${route.route_id}/${capture?.capture_id ?? "missing-capture"}`;
  assert(capture && typeof capture === "object", `${label} capture must be an object.`);
  assert(typeof capture.capture_id === "string" && capture.capture_id.trim(), `${label} capture_id must be non-empty.`);
  assert(Array.isArray(capture.covers) && capture.covers.length > 0, `${label} covers must be non-empty.`);
  assert(capture.covers.every((family) => typeof family === "string" && family.trim()), `${label} covers contains an empty family.`);
  assert(new Set(capture.covers).size === capture.covers.length, `${label} covers contains duplicate families.`);
  assert(capture.activate && typeof capture.activate === "object", `${label} activate must be an object.`);
  assert(typeof capture.activate.result_tree_item_id === "string"
    && SAFE_DOM_ID.test(capture.activate.result_tree_item_id), `${label} activate.result_tree_item_id is invalid.`);
  for (const field of ["table_id", "chart_id"]) {
    if (capture.activate[field] !== undefined) {
      assert(typeof capture.activate[field] === "string" && SAFE_DOM_ID.test(capture.activate[field]), `${label} activate.${field} is invalid.`);
    }
  }
  assert(Array.isArray(capture.observations) && capture.observations.length === capture.covers.length,
    `${label} observations must map one-to-one to covers.`);
  const observationFamilies = [];
  for (const observation of capture.observations) {
    assert(observation && typeof observation === "object", `${label} observation must be an object.`);
    assert(capture.covers.includes(observation.family), `${label} observation family '${observation.family}' is not declared in covers.`);
    assert(OBSERVATION_SOURCES.has(observation.source), `${label} observation source '${observation.source}' is unsupported.`);
    assert(["exact", "contains"].includes(observation.matcher), `${label} observation matcher '${observation.matcher}' is unsupported.`);
    assert(typeof observation.value === "string" && compact(observation.value), `${label} observation value must be a non-empty string.`);
    assert(observation.matcher !== "contains" || compact(observation.value).length >= 4, `${label} contains matcher values must have at least four characters.`);
    observationFamilies.push(observation.family);
  }
  assert(new Set(observationFamilies).size === observationFamilies.length, `${label} observations repeat a family.`);
  assert(deepEqual([...observationFamilies].sort(), [...capture.covers].sort()), `${label} observations do not cover exactly the declared families.`);
}

function inventorySourceRow(inventory, route) {
  const section = route.source.inventory_section;
  assert(["public_methods", "specialized_feature_archives"].includes(section), `${route.route_id} inventory_section is unsupported.`);
  assert(typeof route.source.inventory_key === "string" && route.source.inventory_key.trim(), `${route.route_id} inventory_key is missing.`);
  const key = section === "public_methods" ? "public_kind" : "feature";
  const matches = (inventory[section] ?? []).filter((entry) => entry?.[key] === route.source.inventory_key);
  assert(matches.length === 1, `${route.route_id} must resolve exactly one ${section} row by ${key} '${route.source.inventory_key}'.`);
  return matches[0];
}

function validateInventory(inventory) {
  assert(inventory?.schema === INVENTORY_SCHEMA, `Unexpected reusable inventory schema: ${inventory?.schema ?? "missing"}`);
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
    validateDeclaredArchiveSha256(row);
    assert(typeof row.archive_path === "string", `${row.public_kind} has no archive path.`);
    assert(typeof row.prior_receipt?.path === "string" && typeof row.prior_receipt?.json_pointer === "string", `${row.public_kind} has no prior receipt binding.`);
  }

  const routes = inventory.coverage_routes ?? [];
  assert(Array.isArray(routes) && routes.length >= rows.length, "Inventory v2 coverage_routes must contain at least one route for every public method.");
  const routeIds = routes.map((route) => route?.route_id);
  assert(routeIds.every((routeId) => typeof routeId === "string" && routeId.trim()), "Every coverage route needs a non-empty route_id.");
  assert(new Set(routeIds).size === routeIds.length, "Coverage route IDs are not unique.");
  const allCaptureIds = [];
  const methodFamilies = new Map(kinds.map((kind) => [kind, new Set()]));
  for (const route of routes) {
    assert(kinds.includes(route.public_kind), `${route.route_id} has unknown public_kind '${route.public_kind}'.`);
    assert(route.source && ["inventory", "posthoc_supplement", "named_supplement"].includes(route.source.kind), `${route.route_id} source kind is unsupported.`);
    assert(Array.isArray(route.captures) && route.captures.length > 0, `${route.route_id} must declare at least one capture.`);
    if (route.source.kind === "inventory") {
      const sourceRow = inventorySourceRow(inventory, route);
      assert(sourceRow.public_kind === route.public_kind, `${route.route_id} source public kind differs from its route.`);
      for (const field of ["archive_path", "result_identity", "scientific_identity", "prior_receipt"]) {
        assert(route[field] !== undefined && deepEqual(route[field], sourceRow[field]), `${route.route_id} ${field} must exactly match its referenced inventory row.`);
      }
      if (route.archive_sha256 !== undefined || sourceRow.archive_sha256 !== undefined) {
        assert(route.archive_sha256 === sourceRow.archive_sha256, `${route.route_id} archive_sha256 differs from its referenced inventory row.`);
      }
    } else {
      for (const field of ["archive_path", "result_identity", "scientific_identity", "prior_receipt"]) {
        assert(route[field] === undefined, `${route.route_id} dynamic supplement route must omit ${field}.`);
      }
      if (route.source.kind === "posthoc_supplement") {
        assert(route.public_kind === POSTHOC_KIND, `${route.route_id} posthoc supplement source is bound to the wrong public method.`);
        assert(route.source.case_id === undefined, `${route.route_id} posthoc supplement source must not declare case_id.`);
      } else {
        assert(typeof route.source.case_id === "string" && route.source.case_id.trim(), `${route.route_id} named supplement source requires case_id.`);
      }
    }
    for (const capture of route.captures) {
      validateCaptureDeclaration(route, capture);
      allCaptureIds.push(capture.capture_id);
      const seenFamilies = methodFamilies.get(route.public_kind);
      for (const family of capture.covers) {
        assert(!seenFamilies.has(family), `${route.public_kind} family '${family}' is covered by more than one capture.`);
        seenFamilies.add(family);
      }
    }
  }
  assert(new Set(allCaptureIds).size === allCaptureIds.length, "Capture IDs must be globally unique across coverage_routes.");
  for (const kind of kinds) {
    assert(routes.some((route) => route.public_kind === kind), `${kind} has no coverage route.`);
  }
  return { rows, routes };
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
  validateDeclaredArchiveSha256(supplement);
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

async function loadNamedSupplementReport(file) {
  if (!file) return null;
  const absolute = resolveRepoFile(file, "--named-supplement-report");
  assert(await exists(absolute), `Named supplement report does not exist: ${file}`);
  const report = await readJson(absolute);
  assert(report?.schema_version === 1
    && report?.suite_id === NAMED_SUPPLEMENT_SUITE_ID
    && report?.target_release === TARGET_RELEASE
    && report?.status === "passed"
    && report?.passed === true
    && report?.candidate === "portable"
    && Array.isArray(report?.console_errors)
    && report.console_errors.length === 0, "Named supplement report must be the passed, renderer-clean portable named-case receipt for QuickPLS 2.55.");
  assert(Array.isArray(report.result_archive_supplements), "Named supplement report has no result_archive_supplements array.");
  assert(report.result_archive_supplements.length === 7, `Named supplement report contains ${report.result_archive_supplements.length} result archives instead of seven.`);
  const caseIds = report.result_archive_supplements.map((entry) => entry?.case_id);
  assert(new Set(caseIds).size === caseIds.length, "Named result archive supplement case IDs are not unique.");
  for (let index = 0; index < report.result_archive_supplements.length; index += 1) {
    const supplement = report.result_archive_supplements[index];
    assert(supplement && supplement.status === "passed", `Named archive supplement ${index} is not passed.`);
    assert(typeof supplement.case_id === "string" && supplement.case_id.trim(), `Named archive supplement ${index} has no case_id.`);
    assert(typeof supplement.public_kind === "string" && supplement.public_kind.trim(), `${supplement.case_id} has no public_kind.`);
    validateResultIdentity(supplement.result_identity, supplement.public_kind);
    assert(supplement.scientific_identity && typeof supplement.scientific_identity.method_version === "string"
      && Array.isArray(supplement.scientific_identity.capability_cell_ids), `${supplement.case_id} has no exact scientific identity.`);
    assert(typeof supplement.archive?.path === "string"
      && /^[0-9a-f]{64}$/u.test(supplement.archive?.sha256 ?? "")
      && Number.isInteger(supplement.archive?.size_bytes)
      && supplement.archive.size_bytes > 0, `${supplement.case_id} has no hash-bound archive.`);
    assert(supplement.candidate?.name === "portable"
      && Number.isInteger(supplement.candidate?.pid)
      && typeof supplement.candidate?.path === "string"
      && /^[0-9a-f]{64}$/u.test(supplement.candidate?.sha256 ?? ""), `${supplement.case_id} is not bound to an exact portable candidate process.`);
    assert(typeof supplement.manifest?.path === "string"
      && /^[0-9a-f]{64}$/u.test(supplement.manifest?.sha256 ?? ""), `${supplement.case_id} has no hash-bound named manifest.`);
    const expectedPointer = `#/cases/${jsonPointerIndex(supplement.source_report_json_pointer)}/result_archive_supplement`;
    assert(supplement.source_report_json_pointer === expectedPointer, `${supplement.case_id} source_report_json_pointer is malformed.`);
    const casePointer = supplement.source_report_json_pointer.slice(1);
    const { source_report_json_pointer: _pointer, ...supplementBody } = supplement;
    assert(deepEqual(jsonPointer(report, casePointer), supplementBody), `${supplement.case_id} source report pointer does not bind the exact supplement object.`);
  }
  return {
    absolute,
    relative: slash(path.relative(ROOT, absolute)),
    report,
  };
}

function jsonPointerIndex(pointer) {
  const match = /^#\/cases\/(\d+)\/result_archive_supplement$/u.exec(pointer ?? "");
  assert(match, `Invalid named supplement source report JSON pointer: ${pointer ?? "missing"}`);
  return Number(match[1]);
}

function materializeCoverageRoute({ inventory, route, posthocSupplement, namedSupplementReport }) {
  if (route.source.kind === "inventory") {
    const sourceRow = inventorySourceRow(inventory, route);
    return {
      ...route,
      archive_sha256: route.archive_sha256 ?? sourceRow.archive_sha256,
      source_release: sourceRow.source_release ?? null,
      provenance_source_kind: "inventory",
    };
  }
  if (route.source.kind === "posthoc_supplement") {
    assert(posthocSupplement, `${route.route_id} requires --posthoc-supplement.`);
    assert(posthocSupplement.public_kind === route.public_kind, `${route.route_id} posthoc supplement public kind differs from its route.`);
    return {
      ...route,
      archive_path: posthocSupplement.archive_path,
      archive_sha256: posthocSupplement.archive_sha256,
      result_identity: posthocSupplement.result_identity,
      scientific_identity: posthocSupplement.scientific_identity,
      prior_receipt: posthocSupplement.prior_receipt,
      source_release: TARGET_RELEASE,
      provenance_source_kind: "posthoc_supplement",
    };
  }
  assert(route.source.kind === "named_supplement", `${route.route_id} has an unsupported dynamic source.`);
  assert(namedSupplementReport, `${route.route_id} requires --named-supplement-report.`);
  const matches = namedSupplementReport.report.result_archive_supplements.filter((entry) => entry.case_id === route.source.case_id);
  assert(matches.length === 1, `${route.route_id} must bind exactly one named supplement case '${route.source.case_id}'.`);
  const supplement = matches[0];
  assert(supplement.public_kind === route.public_kind, `${route.route_id} named supplement public kind '${supplement.public_kind}' differs from '${route.public_kind}'.`);
  return {
    ...route,
    archive_path: supplement.archive.path,
    archive_sha256: supplement.archive.sha256,
    result_identity: supplement.result_identity,
    scientific_identity: supplement.scientific_identity,
    prior_receipt: {
      path: namedSupplementReport.relative,
      json_pointer: supplement.source_report_json_pointer,
      verification_status: "passed",
      named_case_id: supplement.case_id,
      source_report_json_pointer: supplement.source_report_json_pointer,
    },
    source_release: TARGET_RELEASE,
    provenance_source_kind: "named_supplement",
    named_supplement_candidate: supplement.candidate,
    named_supplement_manifest: supplement.manifest,
    named_supplement_binding: supplement,
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
    const normalize = (value) => String(value ?? "")
      .trim()
      .replace(/\//g, "\\")
      .replace(/\\+/g, "\\")
      .toLocaleLowerCase("en-US");
    const displayed = document.querySelector(".nd-document-context span")?.textContent ?? "";
    return normalize(displayed) === normalize(target);
  }, archive, { timeout });
}

async function openArchive(page, archive, timeout, methodKind) {
  await runStage(methodKind, "open.packaged_root_reset", () => (
    page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout })
  ));
  await runStage(methodKind, "open.native_shell_ready", () => (
    page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout })
  ));
  await runStage(methodKind, "open.controller_ready", () => page.waitForFunction(() => (
    Boolean(window.__TAURI_INTERNALS__)
    && typeof window.__QUICKPLS_SMOKE__?.setView === "function"
  ), null, { timeout }));
  await runStage(methodKind, "open.path_dispatch", () => page.evaluate((target) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } }));
  }, archive));
  await runStage(methodKind, "open.exact_path_wait", () => waitForOpenedProjectPath(page, archive, timeout));
  await runStage(methodKind, "open.settle", () => page.waitForTimeout(250));
}

function expectedResultOption(identity) {
  return identity.type === "canonical_result_document_id"
    ? `canonical:${identity.value}`
    : identity.value;
}

async function navigateToDeclaredResult(page, row, timeout) {
  await runStage(row.public_kind, "results.surface_dispatch", () => page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "results" } }));
  }));
  await runStage(row.public_kind, "results.surface_wait", () => (
    page.locator('.nd-app[data-surface="results"]').waitFor({ state: "visible", timeout })
  ));
  const workspace = page.locator(".nd-results-workspace");
  await runStage(row.public_kind, "results.workspace_wait", () => workspace.waitFor({ state: "visible", timeout }));
  const select = page.locator(".nd-results-nav .nd-run-select select").first();
  await runStage(row.public_kind, "results.selector_wait", () => select.waitFor({ state: "visible", timeout }));
  const expected = expectedResultOption(row.result_identity);
  await runStage(row.public_kind, "results.declared_option_wait", () => page.waitForFunction((optionValue) => Array.from(
    document.querySelectorAll(".nd-results-nav .nd-run-select select option"),
  ).some((option) => option.value === optionValue), expected, { timeout }));
  await runStage(row.public_kind, "results.declared_option_select", () => select.selectOption(expected));
  await runStage(row.public_kind, "results.selected_option_wait", () => page.waitForFunction((optionValue) => (
    document.querySelector(".nd-results-nav .nd-run-select select")?.value === optionValue
  ), expected, { timeout }));
  await runStage(row.public_kind, "results.document_tab_wait", () => (
    page.locator(".nd-results-document .nd-document-tab").waitFor({ state: "visible", timeout })
  ));

  const canonicalIdentity = await runStage(row.public_kind, "results.canonical_identity_readback", () => (
    page.locator(".nd-canonical-export-v2 details dl > div").evaluateAll((nodes) => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
    return Object.fromEntries(nodes.map((node) => [
      clean(node.querySelector("dt")?.textContent),
      clean(node.querySelector("dd")?.textContent),
    ]));
    })
  ));
  const selectedValue = await runStage(row.public_kind, "results.selected_identity_readback", () => select.inputValue());
  if (row.result_identity.type === "canonical_result_document_id") {
    await runStage(row.public_kind, "results.canonical_identity_assertion", () => {
      assert(canonicalIdentity.Document === row.result_identity.value, `${row.public_kind} selected canonical document '${canonicalIdentity.Document ?? "missing"}' instead of '${row.result_identity.value}'.`);
    });
    await runStage(row.public_kind, "results.canonical_workspace_wait", () => (
      page.locator(".nd-general-sem-canonical-results-workspace").waitFor({ state: "visible", timeout })
    ));
  } else {
    await runStage(row.public_kind, "results.run_identity_assertion", () => {
      assert(selectedValue === row.result_identity.value, `${row.public_kind} selected run '${selectedValue}' instead of '${row.result_identity.value}'.`);
    });
  }
  await runStage(row.public_kind, "results.settle", () => page.waitForTimeout(400));
  return {
    expected_option_value: expected,
    selected_option_value: selectedValue,
    canonical_export_identity: Object.keys(canonicalIdentity).length ? canonicalIdentity : null,
    passed: true,
  };
}

async function activateCaptureTarget(page, row, capture, timeout) {
  const itemId = capture.activate.result_tree_item_id;
  await runStage(row.public_kind, `capture.${capture.capture_id}.expand_tree`, async () => {
    const collapsed = page.locator('.nd-result-tree [role="treeitem"][aria-level="1"][aria-expanded="false"]');
    while (await collapsed.count()) {
      const group = collapsed.first();
      const groupId = await group.getAttribute("data-result-tree-item-id");
      assert(groupId, `${row.public_kind}/${capture.capture_id} found a collapsed result group without an ID.`);
      await group.click({ timeout });
      await page.waitForFunction((targetId) => Array.from(document.querySelectorAll(".nd-result-tree [data-result-tree-item-id]"))
        .some((element) => element.getAttribute("data-result-tree-item-id") === targetId && element.getAttribute("aria-expanded") === "true"), groupId, { timeout });
    }
  });
  const item = page.locator(`[data-result-tree-item-id="${itemId}"]`);
  await runStage(row.public_kind, `capture.${capture.capture_id}.tree_item_wait`, async () => {
    assert(await item.count() === 1, `${row.public_kind}/${capture.capture_id} expected one exact result-tree item '${itemId}'.`);
    await item.waitFor({ state: "visible", timeout });
    assert(await item.getAttribute("aria-level") === "2", `${row.public_kind}/${capture.capture_id} activation target '${itemId}' is not a result leaf.`);
  });
  await runStage(row.public_kind, `capture.${capture.capture_id}.tree_item_activate`, () => item.click({ timeout }));

  if (capture.activate.table_id) {
    const tableId = capture.activate.table_id;
    const table = page.locator(`[data-result-table-id="${tableId}"], [data-canonical-table-id="${tableId}"]`).first();
    await runStage(row.public_kind, `capture.${capture.capture_id}.table_wait`, () => table.waitFor({ state: "visible", timeout }));
  }
  if (capture.activate.chart_id) {
    const chartId = capture.activate.chart_id;
    const chartSelect = page.locator("#nd-canonical-export-v2-chart");
    if (await chartSelect.count() && await chartSelect.locator(`option[value="${chartId}"]`).count()) {
      await runStage(row.public_kind, `capture.${capture.capture_id}.chart_select`, () => chartSelect.selectOption(chartId));
    }
    const exactChart = page.locator(`[data-result-chart-id="${chartId}"], [data-canonical-chart-id="${chartId}"]`).first();
    if (await exactChart.count()) {
      await runStage(row.public_kind, `capture.${capture.capture_id}.chart_wait`, () => exactChart.waitFor({ state: "visible", timeout }));
    } else {
      assert(capture.activate.table_id === chartId, `${row.public_kind}/${capture.capture_id} chart '${chartId}' has neither an exact DOM identity nor a same-ID result table.`);
      await runStage(row.public_kind, `capture.${capture.capture_id}.table_chart_wait`, () => (
        page.locator(`[data-result-table-id="${chartId}"] figure[role="img"], [data-result-table-id="${chartId}"] figure:has(svg[role="img"])`).first()
          .waitFor({ state: "visible", timeout })
      ));
    }
  }
  await runStage(row.public_kind, `capture.${capture.capture_id}.activation_settle`, () => page.waitForTimeout(200));
}

async function observedResultsLabels(page) {
  return page.evaluate(() => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
    const visible = (element) => {
      if (!(element instanceof HTMLElement)) return false;
      const style = getComputedStyle(element);
      return style.display !== "none" && style.visibility !== "hidden" && element.getClientRects().length > 0;
    };
    const unique = (values, limit = 600) => [...new Set(values.map(clean).filter(Boolean))].slice(0, limit);
    const select = document.querySelector(".nd-results-nav .nd-run-select select");
    const selected = select instanceof HTMLSelectElement ? select.selectedOptions[0] : null;
    const workspace = document.querySelector(".nd-results-workspace");
    const visibleTables = Array.from(workspace?.querySelectorAll("table") ?? []).filter(visible);
    const visibleFigures = Array.from(workspace?.querySelectorAll("figure") ?? []).filter(visible);
    return {
      selected_result: clean(selected?.textContent),
      document_tab: clean(document.querySelector(".nd-results-document .nd-document-tab")?.textContent),
      navigation: unique(Array.from(document.querySelectorAll(".nd-result-tree [role='treeitem']")).filter(visible).map((node) => node.textContent)),
      visible_headings: unique(Array.from(workspace?.querySelectorAll("h1, h2, h3, h4") ?? []).filter(visible).map((node) => node.textContent)),
      visible_result_table_ids: unique(Array.from(workspace?.querySelectorAll("[data-result-table-id], [data-canonical-table-id]") ?? []).filter(visible).map((node) => node.getAttribute("data-result-table-id") ?? node.getAttribute("data-canonical-table-id"))),
      table_titles: unique([
        ...Array.from(workspace?.querySelectorAll("[data-result-table-id] h1") ?? []).filter(visible).map((node) => node.textContent),
        ...visibleTables.map((table) => table.querySelector("caption strong")?.textContent),
        ...visibleTables.map((table) => table.querySelector("caption")?.textContent),
      ]),
      table_headers: unique(visibleTables.flatMap((table) => Array.from(table.querySelectorAll("thead th")).map((node) => node.textContent))),
      table_rows: unique(visibleTables.flatMap((table) => Array.from(table.querySelectorAll("tbody tr")).flatMap((row) => [
        row.textContent,
        ...Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent),
      ]))),
      chart_titles: unique(visibleFigures.flatMap((figure) => [
        figure.querySelector("figcaption strong, figcaption h4")?.textContent,
        figure.querySelector("figcaption")?.textContent,
        figure.querySelector("svg[role='img'] title")?.textContent,
        figure.getAttribute("aria-label"),
      ])),
    };
  });
}

function coverAssertionsFromObservation({ row, capture, labels, evidenceIndex, reservedPointers }) {
  const usesTableEvidence = capture.observations.some((observation) => [
    "visible_result_table_ids",
    "table_titles",
    "table_headers",
    "table_rows",
  ].includes(observation.source));
  if (usesTableEvidence) {
    assert(labels.table_headers.length > 0, `${row.public_kind}/${capture.capture_id} table evidence has no visible headers.`);
    assert(labels.table_rows.length > 0, `${row.public_kind}/${capture.capture_id} table evidence has no visible rows.`);
  }
  const byFamily = new Map(capture.observations.map((observation) => [observation.family, observation]));
  return capture.covers.map((family) => {
    const declaration = byFamily.get(family);
    assert(declaration, `${row.public_kind}/${capture.capture_id} has no observation declaration for '${family}'.`);
    const observedValues = labels[declaration.source];
    assert(Array.isArray(observedValues), `${row.public_kind}/${capture.capture_id} source '${declaration.source}' is not an observation array.`);
    const expected = compact(declaration.value);
    const expectedFolded = expected.toLocaleLowerCase("en-US");
    const matchedIndex = observedValues.findIndex((observedValue, index) => {
      const pointer = `/evidence/${evidenceIndex}/observed_results_labels/${declaration.source}/${index}`;
      if (reservedPointers.has(pointer)) return false;
      const observedFolded = observedValue.toLocaleLowerCase("en-US");
      return declaration.matcher === "exact" ? observedFolded === expectedFolded : observedFolded.includes(expectedFolded);
    });
    assert(matchedIndex >= 0, `${row.public_kind}/${capture.capture_id} did not observe '${expected}' via ${declaration.matcher} in ${declaration.source}.`);
    const pointer = `/evidence/${evidenceIndex}/observed_results_labels/${declaration.source}/${matchedIndex}`;
    reservedPointers.add(pointer);
    return {
      family,
      source: declaration.source,
      matcher: declaration.matcher,
      value: expected,
      observed_json_pointer: pointer,
      observed_value: observedValues[matchedIndex],
      passed: true,
    };
  });
}

async function sourceReceiptEvidence(row, staging, cache) {
  const relative = row.prior_receipt.path;
  const absolute = resolveRepoFile(relative, `${row.public_kind} source receipt`);
  assert(await exists(absolute), `${row.public_kind} source receipt is missing: ${relative}`);
  const payload = await readJson(absolute);
  const declaredPointer = row.prior_receipt.json_pointer;
  const resolvedPointer = declaredPointer.startsWith("#/") ? declaredPointer.slice(1) : declaredPointer;
  const observed = jsonPointer(payload, resolvedPointer);
  assert(observed !== undefined, `${row.public_kind} source receipt pointer does not resolve: ${row.prior_receipt.json_pointer}`);
  let directlyBound = observed === row.result_identity.value;
  if (row.provenance_source_kind === "named_supplement") {
    const supplement = row.named_supplement_binding;
    const { source_report_json_pointer: _pointer, ...supplementBody } = supplement;
    directlyBound = deepEqual(observed, supplementBody)
      && observed?.status === "passed"
      && observed?.case_id === row.source.case_id
      && observed?.public_kind === row.public_kind
      && deepEqual(observed?.result_identity, row.result_identity)
      && deepEqual(observed?.scientific_identity, row.scientific_identity)
      && observed?.archive?.path === row.archive_path
      && observed?.archive?.sha256 === row.archive_sha256
      && deepEqual(observed?.candidate, row.named_supplement_candidate)
      && deepEqual(observed?.manifest, row.named_supplement_manifest)
      && row.prior_receipt.source_report_json_pointer === declaredPointer;
  }
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
    json_pointer: declaredPointer,
    resolved_json_pointer: resolvedPointer,
    pointer_value: pointerValueSummary(observed),
    declared_identity_directly_bound: directlyBound,
    identity_recovered_from_archive: archiveRecovered,
  };
}

async function currentScreenshot(page, staging, ordinal, kind, captureId, evidenceIndex) {
  const member = slash(path.join("screenshots", `${String(ordinal).padStart(2, "0")}-${String(evidenceIndex + 1).padStart(2, "0")}-${safeFileName(kind)}-${safeFileName(captureId)}-results.png`));
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

async function archiveIntegrityPostflight({ source, staged, sourceBefore, stagedBefore, archive, row }) {
  const sourceAfter = await fileSha256(source);
  const stagedAfter = await fileSha256(staged);
  const sourceAutosaveAfter = await autosaveSiblingNames(source);
  const stagedAutosaveAfter = await autosaveSiblingNames(staged);
  const integrity = {
    opened_staged_copy: true,
    opened_archive_member: archive.member,
    source_sha256_before: sourceBefore.sha256,
    source_sha256_after: sourceAfter,
    source_sha256_unchanged: sourceAfter === sourceBefore.sha256,
    staged_sha256_before: stagedBefore.sha256,
    staged_sha256_after: stagedAfter,
    staged_sha256_unchanged: stagedAfter === stagedBefore.sha256,
    declared_archive_sha256: row.archive_sha256 ?? null,
    declared_archive_sha256_verified: row.archive_sha256 === undefined ? null : row.archive_sha256 === stagedBefore.sha256,
    source_autosave_siblings_before: sourceBefore.autosaveSiblings,
    source_autosave_siblings_after: sourceAutosaveAfter,
    staged_autosave_siblings_before: stagedBefore.autosaveSiblings,
    staged_autosave_siblings_after: stagedAutosaveAfter,
  };
  assert(integrity.source_sha256_unchanged, `${row.public_kind} source archive changed while its staged copy was open.`);
  assert(integrity.staged_sha256_unchanged, `${row.public_kind} staged archive changed while being inspected.`);
  assert(sourceAutosaveAfter.length === 0, `${row.public_kind} left autosave siblings beside the source archive: ${sourceAutosaveAfter.join(", ")}`);
  assert(stagedAutosaveAfter.length === 0, `${row.public_kind} left autosave siblings beside the staged archive: ${stagedAutosaveAfter.join(", ")}`);
  return integrity;
}

async function crawlMethod({
  page,
  row,
  ordinal,
  staging,
  inventoryPath,
  timeout,
  cache,
  startEvidenceIndex,
  reservedPointers,
}) {
  validateResultIdentity(row.result_identity, row.public_kind);
  validateDeclaredArchiveSha256(row);
  const archiveAbsolute = resolveRepoFile(row.archive_path, `${row.public_kind} archive`);
  assert(await exists(archiveAbsolute), `${row.public_kind} archive is missing: ${row.archive_path}`);
  const sourceBefore = await runStage(row.public_kind, "archive.source_preflight", async () => {
    const sha256 = await fileSha256(archiveAbsolute);
    const autosaveSiblings = await autosaveSiblingNames(archiveAbsolute);
    assert(autosaveSiblings.length === 0, `${row.public_kind} source archive already has autosave siblings: ${autosaveSiblings.join(", ")}`);
    return { sha256, autosaveSiblings };
  });
  const archive = await runStage(row.public_kind, "archive.stage_copy", () => stageArtifact({
    source: archiveAbsolute,
    sourceRelative: row.archive_path,
    directory: "archives",
    staging,
    cache,
  }));
  const stagedArchiveAbsolute = path.join(staging, ...archive.member.split("/"));
  const stagedBefore = await runStage(row.public_kind, "archive.staged_copy_preflight", async () => {
    assert(inside(staging, stagedArchiveAbsolute) && await exists(stagedArchiveAbsolute), `${row.public_kind} staged archive is missing or escaped staging.`);
    const sha256 = await fileSha256(stagedArchiveAbsolute);
    const autosaveSiblings = await autosaveSiblingNames(stagedArchiveAbsolute);
    assert(archive.sha256 === sourceBefore.sha256, `${row.public_kind} staged artifact metadata does not match the source archive hash.`);
    assert(sha256 === archive.sha256, `${row.public_kind} staged archive bytes do not match the source archive hash.`);
    if (row.archive_sha256 !== undefined) {
      assert(row.archive_sha256 === sha256, `${row.public_kind} declared archive_sha256 '${row.archive_sha256}' does not equal staged archive SHA-256 '${sha256}'.`);
    }
    assert(autosaveSiblings.length === 0, `${row.public_kind} staged archive already has autosave siblings: ${autosaveSiblings.join(", ")}`);
    return { sha256, autosaveSiblings };
  });
  const sourceReceipt = await runStage(row.public_kind, "provenance.source_receipt", () => sourceReceiptEvidence(row, staging, cache));

  let captured = [];
  let primaryError = null;
  try {
    await openArchive(page, stagedArchiveAbsolute, timeout, row.public_kind);
    const identityVerification = await navigateToDeclaredResult(page, row, timeout);
    for (let captureIndex = 0; captureIndex < row.captures.length; captureIndex += 1) {
      const capture = row.captures[captureIndex];
      const evidenceIndex = startEvidenceIndex + captureIndex;
      await activateCaptureTarget(page, row, capture, timeout);
      const labels = await runStage(row.public_kind, `capture.${capture.capture_id}.observed_results_labels`, () => observedResultsLabels(page));
      await runStage(row.public_kind, `capture.${capture.capture_id}.required_labels`, () => {
        assert(labels.selected_result, `${row.public_kind}/${capture.capture_id} exposed no selected Results label.`);
        assert(labels.document_tab, `${row.public_kind}/${capture.capture_id} exposed no Results document-tab label.`);
        for (const source of OBSERVATION_SOURCES) {
          assert(Array.isArray(labels[source]), `${row.public_kind}/${capture.capture_id} did not emit the '${source}' observation array.`);
        }
      });
      const coverAssertions = await runStage(row.public_kind, `capture.${capture.capture_id}.cover_assertions`, () => (
        coverAssertionsFromObservation({ row, capture, labels, evidenceIndex, reservedPointers })
      ));
      const screenshot = await runStage(row.public_kind, `capture.${capture.capture_id}.screenshot`, () => (
        currentScreenshot(page, staging, ordinal, row.public_kind, capture.capture_id, evidenceIndex)
      ));
      captured.push({ capture, evidenceIndex, identityVerification, labels, coverAssertions, screenshot });
    }
  } catch (error) {
    primaryError = error;
  }

  let archiveOpenIntegrity = null;
  let integrityError = null;
  try {
    archiveOpenIntegrity = await runStage(row.public_kind, "archive.integrity_postflight", () => archiveIntegrityPostflight({
      source: archiveAbsolute,
      staged: stagedArchiveAbsolute,
      sourceBefore,
      stagedBefore,
      archive,
      row,
    }));
  } catch (error) {
    integrityError = error;
  }
  if (primaryError) {
    const failure = primaryError instanceof Error ? primaryError : new Error(errorMessage(primaryError));
    failure.archiveOpenIntegrity = archiveOpenIntegrity;
    if (integrityError) failure.message = `${failure.message}; postflight also failed: ${errorMessage(integrityError)}`;
    throw failure;
  }
  if (integrityError) throw integrityError;
  assert(captured.length === row.captures.length, `${row.public_kind}/${row.route_id} capture count is incomplete.`);
  return captured.map((entry) => ({
    status: "verified_current_ui_capture",
    route_id: row.route_id,
    capture_id: entry.capture.capture_id,
    provenance_source_kind: row.provenance_source_kind,
    declared_identity: row.result_identity,
    scientific_identity: row.scientific_identity,
    identity_verification: entry.identityVerification,
    archive,
    archive_open_integrity: archiveOpenIntegrity,
    screenshot: entry.screenshot,
    source_receipt: sourceReceipt,
    source_release: row.source_release ?? null,
    source_inventory: inventoryPath,
    observed_results_labels: entry.labels,
    covers: [...entry.capture.covers],
    cover_assertions: entry.coverAssertions,
    current_ui_capture_required: false,
    new_scientific_run_required: false,
  }));
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
  const { rows, routes } = validateInventory(inventory);
  const posthocSupplement = await loadPosthocSupplement(args["posthoc-supplement"]);
  const namedSupplementReport = await loadNamedSupplementReport(args["named-supplement-report"]);
  const connection = await connectToIsolatedPackagedPage(args.endpoint, args.timeout);
  const rendererErrors = observeRendererErrors(connection.page);
  const cache = new Map();
  const methodReceipts = [];
  const failures = [];

  for (let index = 0; index < rows.length; index += 1) {
    const inventoryRow = rows[index];
    const ordinal = index + 1;
    const methodRoutes = routes.filter((route) => route.public_kind === inventoryRow.public_kind);
    const evidence = [];
    const reservedPointers = new Set();
    let receipt;
    try {
      for (const route of methodRoutes) {
        const row = await runStage(inventoryRow.public_kind, `route.${route.route_id}.materialize`, () => materializeCoverageRoute({
          inventory,
          route,
          posthocSupplement,
          namedSupplementReport,
        }));
        const routeEvidence = await crawlMethod({
          page: connection.page,
          row,
          ordinal,
          staging,
          inventoryPath: inventoryRelative,
          timeout: args.timeout,
          cache,
          startEvidenceIndex: evidence.length,
          reservedPointers,
        });
        evidence.push(...routeEvidence);
      }
      assert(evidence.length > 0, `${inventoryRow.public_kind} produced no evidence captures.`);
      const coveredFamilies = evidence.flatMap((entry) => entry.covers);
      assert(new Set(coveredFamilies).size === coveredFamilies.length, `${inventoryRow.public_kind} emitted duplicate family coverage.`);
      assert(reservedPointers.size === coveredFamilies.length, `${inventoryRow.public_kind} did not emit one unique observation pointer per family.`);
      receipt = {
        schema_version: 2,
        suite_id: SUITE_ID,
        target_release: TARGET_RELEASE,
        generated_at: new Date().toISOString(),
        status: "verified_current_ui_capture",
        method_kind: inventoryRow.public_kind,
        source_inventory: inventoryRelative,
        evidence,
      };
    } catch (error) {
      receipt = {
        schema_version: 2,
        suite_id: SUITE_ID,
        target_release: TARGET_RELEASE,
        generated_at: new Date().toISOString(),
        status: "failed",
        method_kind: inventoryRow.public_kind,
        source_inventory: inventoryRelative,
        evidence,
        failure_code: "archive_reopen_or_result_capture_failed",
        failure_stage: errorStage(error),
        failure: errorMessage(error),
      };
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
  assert(methodReceipts.length === 18, `Frozen crawler emitted ${methodReceipts.length} outer method receipts instead of 18.`);
  const manifest = {
    schema_version: 1,
    suite_id: SUITE_ID,
    target_release: TARGET_RELEASE,
    generated_at: new Date().toISOString(),
    status: failures.length ? "failed" : "passed",
    source_inventory: {
      path: inventoryRelative,
      sha256: await fileSha256(inventoryAbsolute),
    },
    posthoc_supplement: args["posthoc-supplement"] ?? null,
    named_supplement_report: args["named-supplement-report"] ?? null,
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
    const firstCapture = firstReceipt.evidence[0];
    manifest.named_evidence_observations.push(await namedObservationFromReceipt({
      staging,
      receiptArtifact: methodReceipts[0],
      caseId: "cross_method:observability:machine-readable observation",
      observed: {
        method_kind: firstReceipt.method_kind,
        capture_id: firstCapture.capture_id,
        selected_result: firstCapture.observed_results_labels.selected_result,
        document_tab: firstCapture.observed_results_labels.document_tab,
        navigation: firstCapture.observed_results_labels.navigation,
        visible_headings: firstCapture.observed_results_labels.visible_headings,
        visible_result_table_ids: firstCapture.observed_results_labels.visible_result_table_ids,
        table_titles: firstCapture.observed_results_labels.table_titles,
        table_headers: firstCapture.observed_results_labels.table_headers,
        table_rows: firstCapture.observed_results_labels.table_rows,
        chart_titles: firstCapture.observed_results_labels.chart_titles,
        covers: firstCapture.covers,
        cover_assertions: firstCapture.cover_assertions,
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
