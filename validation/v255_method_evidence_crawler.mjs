#!/usr/bin/env node
/**
 * Serial QuickPLS 2.55 method evidence crawler.
 *
 * The crawler records every declared Calculate setup case, not merely one
 * selection per public method. Completed Result evidence is deliberately separated
 * into a frozen-archive index: scientific work is not recomputed merely to
 * reproduce a screen, while changed routing/configuration still requires a
 * live calculation journey. Missing or unbound archive evidence is always an
 * explicit release-blocking failure; `--require-results` remains accepted for
 * backwards-compatible invocation but no longer weakens this rule.
 */

import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { startPreview, stopPreview, waitForPreview } from "./lib/v2_ui_smoke_harness.mjs";
import {
  PACKAGED_TAURI_ORIGIN,
  connectToSingleQuickPlsPage,
  observeFunctionalOfflineRequests,
} from "./v247_cdp_package_helpers.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const MATRIX_PATH = path.join(ROOT, "validation", "v255_method_evidence_matrix.json");
const ARCHIVE_INDEX_PATH = path.join(ROOT, "validation", "v255_frozen_result_archive_index.json");
const BUNDLE_MANIFEST_PATH = path.join(ROOT, "validation", "v255_evidence_bundle_manifest.json");
const REUSABLE_ARCHIVE_INVENTORY_PATH = path.join(ROOT, "validation", "v255_reusable_archive_inventory.json");
const RENDERER_ERROR_SETTLE_MS = 250;
const TEST_EVIDENCE_TYPES = new Set([
  "component", "domain", "service", "governance",
  "public_command_interaction", "component_domain_integration", "service_component_integration",
  "public_service_interaction", "store_authority_interaction", "public_domain_contract",
  "store_archive_activation", "public_request_readiness_integration", "public_routing_contract",
  "component_render_contract", "public_projection_contract", "component_registry_integration",
  "public_async_service_interaction", "native_service_boundary", "static_registry_governance",
]);

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function compact(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

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
  if (!new Set(["preview", "packaged", "inventory"]).has(values.mode)) {
    throw new Error("--mode preview, packaged, or inventory is required.");
  }
  values["result-evidence-phase"] ??= values.mode === "packaged" ? "publication" : "source";
  if (!new Set(["source", "publication"]).has(values["result-evidence-phase"])) throw new Error("--result-evidence-phase source or publication is required.");
  if (!values["evidence-dir"]) throw new Error("--evidence-dir is required.");
  if (values.mode !== "inventory" && !values["vitest-report"]) throw new Error("--vitest-report is required outside inventory mode.");
  if (values.mode === "packaged" && !values.endpoint) throw new Error("--endpoint is required for packaged mode.");
  if (values["evidence-bundle"] && !values["evidence-extract-dir"]) throw new Error("--evidence-extract-dir is required with --evidence-bundle.");
  return values;
}

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

async function exists(file) {
  return fs.stat(file).then(() => true, () => false);
}

async function fileSha256(file) {
  return sha256(await fs.readFile(file));
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

function jsonPointer(payload, pointer) {
  assert(typeof pointer === "string" && pointer.startsWith("/"), `Receipt JSON pointer is invalid: ${pointer}`);
  return pointer.slice(1).split("/").reduce((value, segment) => {
    if (value === null || typeof value !== "object") return undefined;
    return value[segment.replace(/~1/g, "/").replace(/~0/g, "~")];
  }, payload);
}

const RESULT_OBSERVATION_SOURCES = new Set(["navigation", "visible_headings", "visible_result_table_ids"]);
const RESULT_OBSERVATION_MATCHERS = new Set(["exact", "contains"]);

function validateObservedResultCoverage(evidence, receiptPayload) {
  const covers = Array.isArray(evidence?.covers) ? evidence.covers : [];
  const assertions = Array.isArray(evidence?.cover_assertions) ? evidence.cover_assertions : [];
  const labels = receiptPayload?.observed_results_labels;
  const result = {
    passed: false,
    covers,
    assertions: [],
    exact_one_assertion_per_family: false,
    assertions_use_unique_observations: false,
    current_results_observations_present: false,
  };
  if (!labels || typeof labels !== "object") return result;
  result.current_results_observations_present = compact(labels.selected_result).length > 0
    && compact(labels.document_tab).length > 0
    && ["navigation", "visible_headings", "visible_result_table_ids"].every((source) => Array.isArray(labels[source]));
  const assertionFamilies = assertions.map((assertion) => assertion?.family);
  result.exact_one_assertion_per_family = assertions.length === covers.length
    && new Set(assertionFamilies).size === assertions.length
    && covers.length === new Set(covers).size
    && covers.every((family) => assertionFamilies.filter((value) => value === family).length === 1);
  const observationKeys = new Set();
  for (const assertion of assertions) {
    const family = assertion?.family;
    const source = assertion?.source;
    const matcher = assertion?.matcher;
    const expected = compact(assertion?.value);
    const observed = RESULT_OBSERVATION_SOURCES.has(source) && Array.isArray(labels[source])
      ? labels[source].map(compact).filter(Boolean)
      : [];
    const normalizedExpected = expected.toLocaleLowerCase("en-US");
    const matched = observed.find((value) => {
      const normalizedValue = value.toLocaleLowerCase("en-US");
      return matcher === "exact"
        ? normalizedValue === normalizedExpected
        : matcher === "contains" && expected.length >= 4 && normalizedValue.includes(normalizedExpected);
    }) ?? null;
    const key = `${source}\0${matcher}\0${normalizedExpected}`;
    const row = {
      family,
      source,
      matcher,
      expected,
      matched_observation: matched,
      passed: covers.includes(family)
        && RESULT_OBSERVATION_SOURCES.has(source)
        && RESULT_OBSERVATION_MATCHERS.has(matcher)
        && expected.length > 0
        && matched !== null
        && !observationKeys.has(key),
    };
    observationKeys.add(key);
    result.assertions.push(row);
  }
  result.assertions_use_unique_observations = observationKeys.size === assertions.length;
  result.passed = result.current_results_observations_present
    && result.exact_one_assertion_per_family
    && result.assertions_use_unique_observations
    && result.assertions.every((assertion) => assertion.passed);
  return result;
}

function normalPath(value) { return String(value ?? "").replace(/\\/g, "/"); }

function vitestAssertions(report) {
  const assertions = [];
  for (const suite of report?.testResults ?? []) {
    for (const assertion of suite?.assertionResults ?? []) {
      assertions.push({
        file: normalPath(suite?.name),
        full_name: String(assertion?.fullName ?? ""),
        title: String(assertion?.title ?? ""),
        status: String(assertion?.status ?? ""),
      });
    }
  }
  return assertions;
}

function exactVitestAssertion(assertions, evidence) {
  const expectedFile = normalPath(evidence.replacement_file).replace(/^\.\//, "");
  const matches = assertions.filter((assertion) => (
    (assertion.file === expectedFile || assertion.file.endsWith(`/${expectedFile}`))
    && assertion.title === evidence.replacement_test
  ));
  return matches.length === 1 ? matches[0] : null;
}

async function verifyHashedEvidence(entry, label, bundleResolver) {
  const result = { label, declared: entry ?? null, passed: false };
  if (!entry || typeof entry.member !== "string" || typeof entry.sha256 !== "string") {
    result.reason = `${label} requires bundle member and sha256.`;
    return result;
  }
  if (!bundleResolver.passed) {
    result.reason = bundleResolver.reason;
    return result;
  }
  const absolute = path.resolve(bundleResolver.extract_dir, entry.member);
  if (!inside(bundleResolver.extract_dir, absolute)) {
    result.reason = `${label} member escapes evidence extraction directory.`;
    return result;
  }
  if (!await exists(absolute)) {
    result.reason = `${label} bundle member is missing from the verified extraction directory.`;
    return result;
  }
  result.member = entry.member;
  result.actual_sha256 = await fileSha256(absolute);
  result.expected_sha256 = entry.sha256.toLowerCase();
  result.passed = /^[a-f0-9]{64}$/i.test(entry.sha256) && result.actual_sha256 === result.expected_sha256;
  if (!result.passed) result.reason = `${label} SHA-256 does not match current bytes.`;
  return result;
}

async function prepareBundleResolver(bundleManifest, args) {
  const resolver = { passed: false, manifest_status: bundleManifest?.status ?? null, release_asset_name: bundleManifest?.bundle?.release_asset_name ?? null };
  if (bundleManifest?.status !== "verified") {
    resolver.reason = "The release-attachable evidence ZIP manifest is pending and is a release blocker.";
    return resolver;
  }
  if (!args["evidence-bundle"] || !args["evidence-extract-dir"]) {
    resolver.reason = "Verified evidence requires --evidence-bundle and --evidence-extract-dir; ignored local validation/results paths are not accepted.";
    return resolver;
  }
  const bundle = path.resolve(args["evidence-bundle"]);
  const extractDir = path.resolve(args["evidence-extract-dir"]);
  if (!await exists(bundle) || !await exists(extractDir)) {
    resolver.reason = "The supplied evidence ZIP or its clean extraction directory is missing.";
    return resolver;
  }
  const expected = bundleManifest?.bundle?.sha256;
  const actual = await fileSha256(bundle);
  if (typeof expected !== "string" || !/^[a-f0-9]{64}$/i.test(expected) || actual !== expected.toLowerCase()) {
    resolver.reason = "The supplied evidence ZIP SHA-256 does not match the curated bundle manifest.";
    return resolver;
  }
  return {
    passed: true,
    bundle: path.relative(ROOT, bundle).split(path.sep).join("/"),
    bundle_sha256: actual,
    extract_dir: extractDir,
    release_asset_name: bundleManifest.bundle.release_asset_name,
  };
}

async function screenshot(page, root, id) {
  const file = path.join(root, "screens", `${id}.png`);
  await fs.mkdir(path.dirname(file), { recursive: true });
  await page.screenshot({ path: file, animations: "disabled" });
  return path.relative(ROOT, file).split(path.sep).join("/");
}

async function namedObservation({ caseId, operation, observed, screenshotPath }) {
  const absolute = path.resolve(ROOT, screenshotPath);
  assert(inside(RESULTS, absolute) && await exists(absolute), `Named evidence screenshot is outside validation/results or missing: ${screenshotPath}`);
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
      path: absolute,
      sha256: await fileSha256(absolute),
    },
  };
}

async function buildNamedEvidenceObservations(report) {
  const observations = [];
  const setup = report.calculate_captures.find((entry) => entry.status === "passed" && typeof entry.screenshot === "string");
  if (setup) {
    observations.push(await namedObservation({
      caseId: "cross_method:observability:setup screenshot",
      operation: "capture_observability_state",
      observed: {
        state: "calculate_setup_visible",
        method_kind: setup.kind,
        executed: setup.executed === true,
        selected_method_assertion_passed: setup.status === "passed",
      },
      screenshotPath: setup.screenshot,
    }));
  }
  if (report.reference_result_surface?.status === "passed" && typeof report.reference_result_surface.screenshot === "string") {
    observations.push(await namedObservation({
      caseId: "cross_method:observability:completed Results screenshot",
      operation: "capture_observability_state",
      observed: {
        state: "completed_results_visible",
        heading: report.reference_result_surface.heading,
        navigation: report.reference_result_surface.navigation,
        visible_result_table_ids: report.reference_result_surface.visible_result_table_ids,
      },
      screenshotPath: report.reference_result_surface.screenshot,
    }));
  }
  if (report.viewport_support?.status === "passed" && typeof report.viewport_support.screenshot === "string") {
    observations.push(await namedObservation({
      caseId: "cross_method:accessibility:1024x700",
      operation: "exercise_accessibility",
      observed: {
        viewport_width: report.viewport_support.width,
        viewport_height: report.viewport_support.height,
        model_surface_visible: report.viewport_support.model_surface_visible,
      },
      screenshotPath: report.viewport_support.screenshot,
    }));
  }
  return observations;
}

function validateMatrix(matrix, archiveIndex) {
  const kinds = (matrix.methods ?? []).map((item) => item.kind);
  const archiveKinds = (archiveIndex.methods ?? []).map((item) => item.kind);
  const setupContract = (matrix.methods ?? []).flatMap((method) => (method.setup_cases ?? []).map((name) => `${method.kind}\0${name}`));
  const specializedContract = matrix.specialized_result_evidence?.required ?? [];
  const crossContract = Object.entries(matrix.cross_method_journeys ?? {}).flatMap(([group, cases]) => (cases ?? []).map((name) => `${group}\0${name}`));
  return {
    matrix_has_exactly_18_methods: kinds.length === 18,
    matrix_method_ids_are_unique: kinds.length === new Set(kinds).size,
    archive_index_has_exactly_18_methods: archiveKinds.length === 18,
    archive_index_method_ids_are_unique: archiveKinds.length === new Set(archiveKinds).size,
    archive_index_matches_matrix: kinds.join("|") === archiveKinds.join("|"),
    serial_execution_is_required: matrix.execution_policy?.maximum_concurrent_calculations === 1,
    archive_reuse_is_explicit: matrix.execution_policy?.reuse_verified_archives_for_presentation_only_evidence === true,
    unverified_archive_evidence_cannot_be_silently_accepted: matrix.execution_policy?.unexplained_skips_allowed === false,
    setup_evidence_schema_is_v2: matrix.schema_version === 2 && (matrix.methods ?? []).every((item) => Array.isArray(item.setup_evidence)),
    result_archive_schema_is_v3: archiveIndex.schema_version === 3 && (archiveIndex.methods ?? []).every((item) => Array.isArray(item.evidence)),
    calculate_capture_has_exactly_18_methods: (matrix.calculate_capture_evidence ?? []).length === 18
      && new Set((matrix.calculate_capture_evidence ?? []).map((item) => item.kind)).size === 18
      && (matrix.calculate_capture_evidence ?? []).map((item) => item.kind).join("|") === kinds.join("|"),
    setup_case_contract_is_frozen: setupContract.length === 64
      && sha256(setupContract.join("\n")) === "4a506f59e5d7afc56b4d975ae567b547b276a8fc841b5a8c9a51836048534fe3"
      && matrix.catalogue_contract?.setup_case_contract_sha256 === "4a506f59e5d7afc56b4d975ae567b547b276a8fc841b5a8c9a51836048534fe3",
    specialized_result_contract_is_frozen: specializedContract.length === 26
      && sha256(specializedContract.join("\n")) === "dfe09e79a231bffe5b88381dd3ee436d6ef7292571bfbdad3662607bb28062ba",
    cross_method_contract_is_frozen: crossContract.length === 29
      && sha256(crossContract.join("\n")) === "e894da0b67cca47224c16fddf8b6e13172c38027967b6ff8906c6cc278ee8ed1",
  };
}

function setupEvidenceContract(matrix) {
  const rows = [];
  for (const method of matrix.methods ?? []) {
    const expected = method.setup_cases ?? [];
    const entries = method.setup_evidence ?? [];
    const declared = entries.map((entry) => entry.setup_case);
    const expectedSet = new Set(expected);
    const declaredSet = new Set(declared);
    rows.push({
      kind: method.kind,
      expected_setup_cases: expected,
      declared_setup_cases: declared,
      exactly_one_entry_per_case: entries.length === expected.length && declaredSet.size === declared.length && expectedSet.size === expected.length && [...expectedSet].every((name) => declaredSet.has(name)),
      entries_use_recognized_evidence_types: entries.every((entry) => TEST_EVIDENCE_TYPES.has(entry.evidence_type) || entry.evidence_type === "browser"),
      entries_are_explicitly_ready_pending_or_verified: entries.every((entry) => ["ready", "pending", "verified"].includes(entry.status)),
    });
  }
  return rows;
}

async function validateArchiveInventory(archiveIndex, bundleResolver) {
  const rows = [];
  for (const item of archiveIndex.methods ?? []) {
    const row = { kind: item.kind, status: item.status, representative_results: item.representative_results ?? [], evidence: [] };
    if (item.status !== "verified") {
      row.passed = false;
      row.reason = "completed-result archive evidence is pending and is a release blocker";
      rows.push(row);
      continue;
    }
    const declaredFamilies = new Set(item.representative_results ?? []);
    const coveredFamilies = new Set();
    for (const [index, evidence] of (item.evidence ?? []).entries()) {
      const entry = { index, covers: evidence.covers ?? [] };
      entry.method_kind_matches_row = typeof evidence.method_kind === "string" && evidence.method_kind === item.kind;
      entry.canonical_result_id_present = typeof evidence.canonical_result_id === "string" && evidence.canonical_result_id.trim().length > 0;
      entry.covers_declared_families_only = Array.isArray(evidence.covers) && evidence.covers.length > 0 && evidence.covers.every((family) => declaredFamilies.has(family));
      entry.archive = await verifyHashedEvidence(evidence.archive, "archive", bundleResolver);
      entry.screenshot = await verifyHashedEvidence(evidence.screenshot, "screenshot", bundleResolver);
      entry.receipt = await verifyHashedEvidence(evidence.receipt, "receipt", bundleResolver);
      entry.receipt_parseable = false;
      entry.receipt_binding = { passed: false };
      entry.receipt_current_ui_observation = { passed: false };
      entry.coverage_observations = { passed: false };
      if (entry.receipt.passed && typeof evidence.receipt?.method_kind_json_pointer === "string" && typeof evidence.receipt?.canonical_result_id_json_pointer === "string") {
        try {
          const receiptPayload = JSON.parse(await fs.readFile(path.resolve(bundleResolver.extract_dir, evidence.receipt.member), "utf8"));
          entry.receipt_parseable = true;
          const receiptMethodKind = jsonPointer(receiptPayload, evidence.receipt.method_kind_json_pointer);
          const receiptCanonicalResultId = jsonPointer(receiptPayload, evidence.receipt.canonical_result_id_json_pointer);
          entry.receipt_binding = { method_kind: receiptMethodKind, canonical_result_id: receiptCanonicalResultId, passed: receiptMethodKind === evidence.method_kind && receiptCanonicalResultId === evidence.canonical_result_id };
          entry.receipt_current_ui_observation = {
            schema_version: receiptPayload?.schema_version ?? null,
            suite_id: receiptPayload?.suite_id ?? null,
            target_release: receiptPayload?.target_release ?? null,
            status: receiptPayload?.status ?? null,
            passed: receiptPayload?.schema_version === 1
              && receiptPayload?.suite_id === "quickpls_v255_frozen_archive_reopen_crawler_v1"
              && receiptPayload?.target_release === "2.55.0"
              && receiptPayload?.status === "verified_current_ui_capture"
              && receiptPayload?.method_kind === evidence.method_kind
              && receiptPayload?.archive?.member === evidence.archive?.member
              && receiptPayload?.archive?.sha256 === evidence.archive?.sha256
              && receiptPayload?.screenshot?.member === evidence.screenshot?.member
              && receiptPayload?.screenshot?.sha256 === evidence.screenshot?.sha256,
          };
          entry.coverage_observations = validateObservedResultCoverage(evidence, receiptPayload);
          if (entry.coverage_observations.passed) {
            for (const family of evidence.covers ?? []) coveredFamilies.add(family);
          }
        } catch (error) { entry.receipt_parse_error = error instanceof Error ? error.message : String(error); }
      }
      entry.passed = entry.method_kind_matches_row
        && entry.canonical_result_id_present
        && entry.covers_declared_families_only
        && entry.archive.passed
        && entry.screenshot.passed
        && entry.receipt.passed
        && entry.receipt_parseable
        && entry.receipt_binding.passed
        && entry.receipt_current_ui_observation.passed
        && entry.coverage_observations.passed;
      row.evidence.push(entry);
    }
    row.coverage = {
      declared: [...declaredFamilies],
      covered: [...coveredFamilies],
      exact: declaredFamilies.size === coveredFamilies.size && [...declaredFamilies].every((family) => coveredFamilies.has(family)),
    };
    row.passed = row.coverage.exact && row.evidence.length > 0 && row.evidence.every((entry) => entry.passed);
    if (!row.passed) row.reason = "verified archive evidence is incomplete, tampered, unparseable, unbound, or does not exactly cover every declared result family";
    rows.push(row);
  }
  return rows;
}

async function validateReusableArchiveInventory(inventory, archiveIndex) {
  const expectedKinds = (archiveIndex.methods ?? []).map((item) => item.kind);
  const records = inventory?.public_methods ?? [];
  const byKind = new Map(records.map((item) => [item.public_kind, item]));
  const rows = [];
  for (const kind of expectedKinds) {
    const item = byKind.get(kind);
    const reusable = item?.reuse_state === "reusable_verified_prior_release";
    const requiresNewRun = item?.new_scientific_run_required === true;
    const row = { kind, reuse_state: item?.reuse_state ?? "missing", source_release: item?.source_release ?? null, passed: false };
    if (!item) row.reason = "No reusable archive inventory record exists.";
    else if (requiresNewRun) row.reason = "This method requires a new packaged scientific run before publication evidence can be built.";
    else if (!reusable) row.reason = "Reusable archive status is not established.";
    else {
      const files = [item.archive_path, item.prior_receipt?.path].filter((value) => typeof value === "string");
      row.files = [];
      for (const relative of files) {
        const absolute = path.resolve(ROOT, relative);
        const entry = { path: relative, exists: inside(ROOT, absolute) && await exists(absolute), sha256: null };
        if (entry.exists) entry.sha256 = await fileSha256(absolute);
        row.files.push(entry);
      }
      row.prior_screenshot_count = Array.isArray(item.prior_screenshots) ? item.prior_screenshots.length : 0;
      row.current_ui_capture_required = item.current_ui_capture_required === true;
      row.passed = files.length === 2 && row.files.every((file) => file.exists);
      if (!row.passed) row.reason = "Reusable archive or receipt cannot be read and hashed from the current workspace.";
    }
    rows.push(row);
  }
  return {
    schema_version: inventory?.schema ?? null,
    rows,
    reusable_count: rows.filter((row) => row.reuse_state === "reusable_verified_prior_release" && row.passed).length,
    later_new_packaged_runs: rows.filter((row) => row.reason?.startsWith("This method requires")).map((row) => row.kind),
  };
}

async function openCalculateForFixture(page, fixtureName) {
  await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture === "function", null, { timeout: 20_000 });
  await page.evaluate((requested) => window.__QUICKPLS_SMOKE__?.loadDiagramFixture(requested), fixtureName);
  await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout: 20_000 });
  await page.keyboard.press("Control+R");
  const dialog = page.getByRole("dialog", { name: "Calculate", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 20_000 });
  const options = dialog.locator('#nd-calculation-method-list [role="option"]');
  await page.waitForFunction(
    () => document.querySelectorAll('#nd-calculation-method-list [role="option"]').length === 18,
    null,
    { timeout: 20_000 },
  );
  const visibleCount = await options.count();
  assert(visibleCount === 18, `Calculate exposed ${visibleCount} methods instead of 18.`);
  return dialog;
}

async function executeBrowserSetupCase(page, { method, caseName, evidence, evidenceRoot, index }) {
  const fixture = evidence.controlled_fixture;
  assert(fixture && typeof fixture.smoke_fixture === "string" && fixture.smoke_fixture.trim(), `${method.kind} / ${caseName} lacks a controlled smoke fixture.`);
  const requiredText = fixture.required_text ?? [];
  const requiredSelectors = fixture.required_selectors ?? [];
  assert((Array.isArray(requiredText) && requiredText.length > 0) || (Array.isArray(requiredSelectors) && requiredSelectors.length > 0), `${method.kind} / ${caseName} must declare an observable case assertion.`);
  const dialog = await openCalculateForFixture(page, fixture.smoke_fixture);
  const option = dialog.locator(`#nd-calculation-method-${method.kind}`);
  await option.waitFor({ state: "visible", timeout: 10_000 });
  await option.click();
  assert(await option.getAttribute("aria-selected") === "true", `${method.kind} did not remain selected after click.`);
  const panel = dialog.locator("#nd-calculation-panel");
  await panel.waitFor({ state: "visible", timeout: 10_000 });
  const content = compact(await panel.textContent());
  for (const required of requiredText) assert(content.includes(required), `${method.kind} / ${caseName} lacks required evidence '${required}'.`);
  for (const selector of requiredSelectors) await panel.locator(selector).first().waitFor({ state: "visible", timeout: 10_000 });
  const captured = await screenshot(page, evidenceRoot, `${String(index + 1).padStart(3, "0")}-setup-${method.kind}-${caseName.replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "").toLowerCase()}`);
  return {
    status: "passed",
    executed: true,
    screenshot: captured,
    controlled_fixture: fixture,
    assertion_identity: { required_text: requiredText, required_selectors: requiredSelectors },
  };
}

function executeTestSetupCase(evidence, assertions) {
  const assertion = exactVitestAssertion(assertions, evidence);
  const base = {
    executed: Boolean(assertion),
    replacement_file: evidence.replacement_file,
    replacement_test: evidence.replacement_test,
    assertion_identity: assertion ? { file: assertion.file, full_name: assertion.full_name, title: assertion.title, status: assertion.status } : null,
  };
  if (!assertion) return { ...base, status: "failed", reason: "The exact named setup assertion is absent from the Vitest JSON report." };
  if (assertion.status !== "passed") return { ...base, status: "failed", reason: `The exact named setup assertion reported '${assertion.status}'.` };
  return { ...base, status: "passed" };
}

async function runSetupCrawler(page, { evidenceRoot, matrix, report, assertions }) {
  let ordinal = 0;
  for (const method of matrix.methods) {
    const declarations = new Map((method.setup_evidence ?? []).map((entry) => [entry.setup_case, entry]));
    for (const caseName of method.setup_cases ?? []) {
      ordinal += 1;
      const evidence = declarations.get(caseName);
      const record = { kind: method.kind, setup_case: caseName, status: "pending", executed: false, evidence_type: evidence?.evidence_type ?? null };
      if (!evidence) {
        record.reason = "No one-to-one setup evidence declaration exists for this required setup case.";
      } else if (!["ready", "verified"].includes(evidence.status)) {
        record.reason = evidence.reason ?? "Setup evidence is pending and is a release blocker.";
      } else if (TEST_EVIDENCE_TYPES.has(evidence.evidence_type)) {
        Object.assign(record, executeTestSetupCase(evidence, assertions));
      } else if (evidence.evidence_type === "browser") {
        try {
          Object.assign(record, await executeBrowserSetupCase(page, { method, caseName, evidence, evidenceRoot, index: ordinal }));
        } catch (error) {
          record.status = "failed";
          record.executed = true;
          record.reason = error instanceof Error ? error.message : String(error);
          record.screenshot = await screenshot(page, evidenceRoot, `${String(ordinal).padStart(3, "0")}-setup-failure-${method.kind}`).catch(() => null);
        }
      } else {
        record.reason = "Setup evidence type must be browser, component, domain, service, or governance.";
      }
      report.setups.push(record);
    }
  }
  const expected = matrix.methods.flatMap((method) => method.setup_cases.map((setupCase) => `${method.kind}\u0000${setupCase}`));
  const recorded = report.setups.map((entry) => `${entry.kind}\u0000${entry.setup_case}`);
  assert(recorded.length === expected.length && recorded.every((entry, index) => entry === expected[index]), "Every declared setup case must be independently recorded in matrix order.");
}

async function runCalculateCaptures(page, { evidenceRoot, matrix, report }) {
  const methodsByKind = new Map(matrix.methods.map((method) => [method.kind, method]));
  for (const [index, evidence] of (matrix.calculate_capture_evidence ?? []).entries()) {
    const method = methodsByKind.get(evidence.kind);
    const record = { kind: evidence.kind, status: "pending", executed: false, evidence_type: evidence.evidence_type ?? null };
    if (!method || !["ready", "verified"].includes(evidence.status) || evidence.evidence_type !== "browser") {
      record.reason = "A Calculate capture requires a ready browser fixture for one known public method.";
    } else {
      try {
        Object.assign(record, await executeBrowserSetupCase(page, { method, caseName: "calculate-capture", evidence, evidenceRoot, index }));
      } catch (error) {
        record.status = "failed";
        record.executed = true;
        record.reason = error instanceof Error ? error.message : String(error);
        record.screenshot = await screenshot(page, evidenceRoot, `${String(index + 1).padStart(3, "0")}-calculate-capture-failure-${method.kind}`).catch(() => null);
      }
    }
    report.calculate_captures.push(record);
  }
}

async function captureRepresentativeResult(page, { evidenceRoot, report, origin }) {
  await page.goto(`${origin}/?quickpls_smoke=completed`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.setView === "function", null, { timeout: 20_000 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("results"));
  const results = page.locator(".nd-results-workspace");
  await results.waitFor({ state: "visible", timeout: 20_000 });
  const captured = await screenshot(page, evidenceRoot, "results-reference-surface");
  const observed = await page.evaluate(() => {
    const clean = (value) => String(value ?? "").replace(/\s+/g, " ").trim();
    const unique = (values) => [...new Set(values.map(clean).filter(Boolean))];
    return {
      navigation: unique(Array.from(document.querySelectorAll(".nd-result-tree [role='treeitem']")).map((node) => node.textContent)),
      visible_result_table_ids: unique(Array.from(document.querySelectorAll("[data-result-table-id], [data-canonical-table-id]")).map((node) => node.getAttribute("data-result-table-id") ?? node.getAttribute("data-canonical-table-id"))),
    };
  });
  report.reference_result_surface = {
    screenshot: captured,
    heading: compact(await results.locator("h1, h2, .nd-pane-title").first().textContent()),
    navigation: observed.navigation,
    visible_result_table_ids: observed.visible_result_table_ids,
    status: "passed",
    note: "This is a navigation/rendering reference only. Per-method completed Result evidence is controlled by the archive index.",
  };
}

async function captureViewportSupport(page, { evidenceRoot, report }) {
  await page.setViewportSize({ width: 1024, height: 700 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("model"));
  const modelSurface = page.locator('.nd-app[data-native-desktop-shell="true"]');
  await modelSurface.waitFor({ state: "visible", timeout: 20_000 });
  const viewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
  assert(viewport.width === 1024 && viewport.height === 700, `Expected a 1024x700 viewport, observed ${viewport.width}x${viewport.height}.`);
  report.viewport_support = {
    ...viewport,
    model_surface_visible: await modelSurface.isVisible(),
    screenshot: await screenshot(page, evidenceRoot, "viewport-1024x700-model"),
    status: "passed",
  };
}

async function runBrowserCrawler(args, matrix, archiveIndex, report, assertions) {
  let browser;
  let preview;
  let offline;
  let rendererErrors;
  try {
    let page;
    let origin;
    if (args.mode === "preview") {
      const port = Number(args.port ?? 57656);
      assert(Number.isInteger(port) && port > 0 && port < 65536, "--port must be a valid TCP port.");
      preview = startPreview(port);
      origin = `http://127.0.0.1:${port}`;
      await waitForPreview(origin, preview.logs);
      browser = await chromium.launch({ headless: true });
      page = await browser.newPage({ viewport: { width: 1024, height: 700 }, deviceScaleFactor: 1 });
      rendererErrors = observeRendererErrors(page);
      report.runtime = "production frontend preview";
      await page.goto(`${origin}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    } else {
      const connection = await connectToSingleQuickPlsPage({ chromium, endpoint: args.endpoint });
      browser = connection.browser;
      page = connection.page;
      rendererErrors = observeRendererErrors(page);
      offline = observeFunctionalOfflineRequests(page);
      report.runtime = "packaged Tauri WebView2 over isolated local CDP";
      origin = PACKAGED_TAURI_ORIGIN;
      await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    }
    await runCalculateCaptures(page, { evidenceRoot: args.evidenceDir, matrix, report });
    await runSetupCrawler(page, { evidenceRoot: args.evidenceDir, matrix, report, assertions });
    await captureRepresentativeResult(page, { evidenceRoot: args.evidenceDir, report, origin });
    await captureViewportSupport(page, { evidenceRoot: args.evidenceDir, report });
    await rendererErrors.settle();
    report.console_errors = [...rendererErrors.errors];
    report.offline = offline?.summary() ?? { passed: true, mode: "preview" };
    assert(report.console_errors.length === 0, `Crawler observed console errors: ${JSON.stringify(report.console_errors)}`);
    assert(report.offline.passed, `Packaged crawler accessed an external origin: ${JSON.stringify(report.offline)}`);
  } finally {
    if (rendererErrors) {
      report.console_errors = [...rendererErrors.errors];
      rendererErrors.stop();
    }
    offline?.stop();
    if (preview) await browser?.close().catch(() => undefined);
    if (preview) stopPreview(preview.server, Number(args.port ?? 57656));
  }
}

const args = parseArgs(process.argv.slice(2));
args.evidenceDir = path.resolve(args["evidence-dir"]);
assert(inside(RESULTS, args.evidenceDir), "--evidence-dir must remain below validation/results.");
if (await exists(args.evidenceDir)) throw new Error(`Refusing to reuse evidence directory: ${args.evidenceDir}`);
await fs.mkdir(args.evidenceDir, { recursive: true });

const matrix = await readJson(MATRIX_PATH);
const archiveIndex = await readJson(ARCHIVE_INDEX_PATH);
const bundleManifest = await readJson(BUNDLE_MANIFEST_PATH);
const reusableArchiveInventory = await readJson(REUSABLE_ARCHIVE_INVENTORY_PATH);
const vitestReportPath = args["vitest-report"] ? path.resolve(args["vitest-report"]) : null;
if (vitestReportPath) assert(inside(RESULTS, vitestReportPath), "--vitest-report must remain below validation/results.");
const vitestReport = vitestReportPath ? await readJson(vitestReportPath) : null;
const assertions = vitestAssertions(vitestReport);
const matrixChecks = validateMatrix(matrix, archiveIndex);
const setupContracts = setupEvidenceContract(matrix);
const bundleResolver = await prepareBundleResolver(bundleManifest, args);
const reusableArchiveCheck = await validateReusableArchiveInventory(reusableArchiveInventory, archiveIndex);
const report = {
  schema_version: 2,
  suite_id: "quickpls_v255_method_evidence_crawler_v2",
  target_release: "2.55.0",
  version_authority_at_collection: args.mode === "packaged"
    ? "2.55.0 candidate after the consolidated source gate"
    : "2.54.0 source-gate authority before the release bump",
  generated_at: new Date().toISOString(),
  mode: args.mode,
  result_evidence_phase: args["result-evidence-phase"],
  serial: true,
  maximum_concurrent_calculations: 1,
  passed: false,
  console_errors: [],
  matrix_checks: matrixChecks,
  setup_evidence_contract: setupContracts,
  sources: {
    matrix_sha256: sha256(await fs.readFile(MATRIX_PATH)),
    frozen_archive_index_sha256: sha256(await fs.readFile(ARCHIVE_INDEX_PATH)),
    evidence_bundle_manifest_sha256: sha256(await fs.readFile(BUNDLE_MANIFEST_PATH)),
    reusable_archive_inventory_sha256: sha256(await fs.readFile(REUSABLE_ARCHIVE_INVENTORY_PATH)),
    vitest_report: vitestReportPath ? path.relative(ROOT, vitestReportPath).split(path.sep).join("/") : null,
    vitest_report_sha256: vitestReportPath ? await fileSha256(vitestReportPath) : null,
  },
  setups: [],
  calculate_captures: [],
  archive_inventory: [],
  evidence_bundle: bundleResolver,
  reusable_archive_inventory: reusableArchiveCheck,
  viewport_support: null,
  named_evidence_observations: [],
  failures: [],
};
try {
  assert(Object.values(matrixChecks).every(Boolean), `Invalid 2.55 evidence matrix: ${JSON.stringify(matrixChecks)}`);
  report.archive_inventory = await validateArchiveInventory(archiveIndex, bundleResolver);
  if (args.mode !== "inventory") await runBrowserCrawler(args, matrix, archiveIndex, report, assertions);
  const publicationEvidencePassed = report.archive_inventory.every((entry) => entry.passed);
  const sourceInventoryPassed = reusableArchiveCheck.reusable_count === 17
    && reusableArchiveCheck.later_new_packaged_runs.length === 1
    && reusableArchiveCheck.later_new_packaged_runs[0] === "pls_posthoc_technical_minimum_sample_size";
  const expectedSetupCount = matrix.methods.reduce((count, method) => count + method.setup_cases.length, 0);
  const setupPassed = args.mode === "inventory" || (report.setups.length === expectedSetupCount && report.setups.every((entry) => entry.status === "passed"));
  const calculateCapturesPassed = args.mode === "inventory" || (report.calculate_captures.length === 18 && report.calculate_captures.every((entry) => entry.status === "passed" && typeof entry.screenshot === "string"));
  const setupContractPassed = setupContracts.every((entry) => entry.exactly_one_entry_per_case && entry.entries_use_recognized_evidence_types && entry.entries_are_explicitly_ready_pending_or_verified);
  report.passed = Object.values(matrixChecks).every(Boolean)
    && setupPassed
    && calculateCapturesPassed
    && setupContractPassed
    && (args["result-evidence-phase"] === "publication" ? publicationEvidencePassed : sourceInventoryPassed);
  if (report.passed && args.mode !== "inventory") {
    report.named_evidence_observations = await buildNamedEvidenceObservations(report);
  }
  if (!report.passed) report.failures.push(args["result-evidence-phase"] === "publication"
    ? "Required method setup or final ZIP-bound Result evidence is incomplete."
    : "Required method setup evidence or the 17-reusable/one-new-run source archive inventory is incomplete.");
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error));
} finally {
  report.completed_at = new Date().toISOString();
  await fs.writeFile(path.join(args.evidenceDir, "v255_method_evidence_crawler.json"), `${JSON.stringify(report, null, 2)}\n`, "utf8");
}
if (!report.passed) {
  console.error(report.failures[0] ?? "QuickPLS 2.55 method evidence crawler failed.");
  process.exit(1);
}
await new Promise((resolve, reject) => {
  process.stdout.write(`${JSON.stringify({ passed: true, setups: report.setups.length, archive_rows: report.archive_inventory.length }, null, 2)}\n`, (error) => {
    if (error) reject(error);
    else resolve();
  });
});
// In packaged mode this process owns only the Playwright CDP client. Exiting
// locally detaches that client without sending Browser.close to the
// wrapper-owned QuickPLS/WebView process, whose exact PID tree remains under
// the PowerShell supervisor's authority.
process.exit(0);
