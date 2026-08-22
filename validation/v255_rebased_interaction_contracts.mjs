#!/usr/bin/env node
/**
 * QuickPLS 2.55 interaction-first replacement-contract harness.
 *
 * Each REB-001…REB-017 entry is executed and recorded independently. A case
 * can share browser setup with another case, but it can never inherit another
 * case's success. Missing controlled fixtures are recorded as `pending` and
 * fail the suite; selecting a calculation method is not evidence for an
 * unrelated archive, dirty-work, modal, or result-authority contract.
 */

import fs from "node:fs/promises";
import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { startPreview, stopPreview, waitForPreview } from "./lib/v2_ui_smoke_harness.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const REBASELINE = path.join(ROOT, "validation", "v255_regression_rebaseline.json");
const MATRIX = path.join(ROOT, "validation", "v255_method_evidence_matrix.json");
const DEFAULT_FIXTURES = path.join(ROOT, "validation", "v255_interaction_fixture_manifest.json");
// These names are deliberately closed rather than treating an unfamiliar
// label as a browser case. Every one means one exact Vitest assertion identity.
const TEST_EVIDENCE_TYPES = new Set([
  "public_command_interaction", "component_domain_integration", "service_component_integration",
  "public_service_interaction", "store_authority_interaction", "public_domain_contract",
  "store_archive_activation", "public_request_readiness_integration", "public_routing_contract",
  "component_render_contract", "public_projection_contract", "component_registry_integration",
  "public_async_service_interaction", "native_service_boundary", "static_registry_governance",
]);

function assert(condition, message) { if (!condition) throw new Error(message); }
function compact(value) { return String(value ?? "").replace(/\s+/g, " ").trim(); }
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
  if (!values.mode || !new Set(["contract", "browser"]).has(values.mode)) throw new Error("--mode contract or --mode browser is required.");
  if (values.mode === "browser" && !values["evidence-dir"]) throw new Error("--evidence-dir is required for browser mode.");
  if (values.mode === "browser" && !values["vitest-report"]) throw new Error("--vitest-report is required for browser mode.");
  return values;
}

async function readJson(file) { return JSON.parse(await fs.readFile(file, "utf8")); }
async function sha256File(file) { return crypto.createHash("sha256").update(await fs.readFile(file)).digest("hex"); }
async function capture(page, root, id) {
  const file = path.join(root, "screens", `${id}.png`);
  await fs.mkdir(path.dirname(file), { recursive: true });
  await page.screenshot({ path: file, animations: "disabled" });
  return path.relative(ROOT, file).split(path.sep).join("/");
}

function validateContract(rebaseline, matrix, fixtureManifest) {
  const items = rebaseline.items ?? [];
  const ids = items.map((item) => item.id);
  const fixtureIds = (fixtureManifest.cases ?? []).map((item) => item.id);
  const kinds = (matrix.methods ?? []).map((item) => item.kind);
  return {
    expected_replacement_count: items.length === 17,
    replacement_ids_unique: ids.length === new Set(ids).size,
    every_item_declares_an_interaction_route: items.every((item) => Array.isArray(item.interaction_route) && item.interaction_route.length > 0),
    every_item_is_addressable_by_the_browser_harness: items.every((item) => item.harness_case === item.id),
    fixture_manifest_has_one_case_per_browser_rebaseline_item: items.filter((item) => item.evidence_type === "browser").every((item) => fixtureIds.includes(item.id)),
    every_rebaseline_row_declares_one_evidence_type: items.every((item) => TEST_EVIDENCE_TYPES.has(item.evidence_type) || item.evidence_type === "browser"),
    test_backed_rows_name_a_current_replacement_test: items.filter((item) => TEST_EVIDENCE_TYPES.has(item.evidence_type)).every((item) => typeof item.replacement_file === "string" && item.replacement_file.trim() && typeof item.replacement_test === "string" && item.replacement_test.trim()),
    browser_rows_name_a_controlled_browser_fixture: items.filter((item) => item.evidence_type === "browser").every((item) => fixtureIds.includes(item.id)),
    source_literal_assertions_are_not_the_replacement_policy: rebaseline.policy?.replacement_evidence === "interaction_first",
    exact_18_method_contract_is_retained: rebaseline.policy?.preserve_public_calculate_method_count === 18 && kinds.length === 18,
    matrix_methods_are_unique: kinds.length === new Set(kinds).size,
  };
}

function normalPath(value) { return String(value ?? "").replace(/\\/g, "/"); }

function vitestAssertions(report) {
  const entries = [];
  for (const suite of report?.testResults ?? []) {
    for (const assertion of suite?.assertionResults ?? []) {
      entries.push({
        file: normalPath(suite?.name),
        full_name: String(assertion?.fullName ?? ""),
        title: String(assertion?.title ?? ""),
        status: String(assertion?.status ?? ""),
      });
    }
  }
  return entries;
}

function matchingVitestAssertions(assertions, item) {
  const expectedFile = normalPath(item.replacement_file).replace(/^\.\//, "");
  return assertions.filter((assertion) => (
    (assertion.file === expectedFile || assertion.file.endsWith(`/${expectedFile}`))
    && assertion.title === item.replacement_test
  ));
}

function executeTestBackedCase(item, assertions) {
  const matches = matchingVitestAssertions(assertions, item);
  const assertion = matches.length === 1 ? matches[0] : null;
  const base = {
    id: item.id,
    route: item.interaction_route,
    contract: item.replacement_contract,
    evidence_type: item.evidence_type,
    executed: Boolean(assertion),
    replacement_file: item.replacement_file,
    replacement_test: item.replacement_test,
    assertion_identity: assertion ? { file: assertion.file, full_name: assertion.full_name, title: assertion.title, status: assertion.status } : null,
    started_at: new Date().toISOString(),
    completed_at: new Date().toISOString(),
  };
  if (!assertion) return { ...base, status: "failed", reason: matches.length ? "The named replacement assertion is ambiguous within its declared test file." : "The named replacement assertion was not present in the Vitest JSON report." };
  if (assertion.status !== "passed") return { ...base, status: "failed", reason: `The named replacement assertion reported '${assertion.status}'.` };
  return { ...base, status: "passed" };
}

async function ensureCalculate(page) {
  const dialog = page.getByRole("dialog", { name: "Calculate", exact: true });
  if (!await dialog.isVisible().catch(() => false)) {
    await page.keyboard.press("Control+R");
    await dialog.waitFor({ state: "visible", timeout: 15_000 });
  }
  const options = dialog.locator('#nd-calculation-method-list [role="option"]');
  assert(await options.count() === 18, `Calculate exposed ${await options.count()} methods instead of 18.`);
  return dialog;
}

async function loadSmokeFixture(page, fixture) {
  const name = fixture.smoke_fixture ?? "large";
  await page.waitForFunction(() => typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture === "function", null, { timeout: 15_000 });
  await page.evaluate((requested) => window.__QUICKPLS_SMOKE__?.loadDiagramFixture(requested), name);
  await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout: 15_000 });
}

async function selectMethod(page, kind) {
  const dialog = await ensureCalculate(page);
  const option = dialog.locator(`#nd-calculation-method-${kind}`);
  await option.waitFor({ state: "visible", timeout: 10_000 });
  await option.focus();
  await page.keyboard.press("Enter");
  assert(await option.getAttribute("aria-selected") === "true", `${kind} did not become selected through keyboard interaction.`);
  return dialog;
}

async function requireFixtureFile(fixture, key) {
  const relative = fixture[key];
  assert(typeof relative === "string" && relative.trim(), `Fixture requires '${key}'.`);
  const absolute = path.resolve(ROOT, relative);
  const relativeToRoot = path.relative(ROOT, absolute);
  assert(!relativeToRoot.startsWith("..") && !path.isAbsolute(relativeToRoot), `Fixture '${key}' escapes the repository.`);
  await fs.access(absolute);
  return absolute;
}

async function executeCanvasEnterEdit(page, fixture) {
  await loadSmokeFixture(page, fixture);
  for (const selector of fixture.required_selectors ?? []) {
    const target = page.locator(selector).first();
    await target.waitFor({ state: "visible", timeout: 10_000 });
    await target.click();
    await page.keyboard.press("Enter");
    await page.getByRole("dialog").first().waitFor({ state: "visible", timeout: 5_000 });
    await page.keyboard.press("Escape");
  }
  assert((fixture.required_selectors ?? []).length >= 4, "Canvas Enter fixture must cover construct, path, HOC, and moderation targets.");
}

async function executeCalculateRegistry(page, fixture) {
  await loadSmokeFixture(page, fixture);
  const dialog = await selectMethod(page, fixture.method_kind ?? "cbsem");
  const panel = dialog.locator("#nd-calculation-panel");
  await panel.waitFor({ state: "visible", timeout: 10_000 });
  const text = compact(await panel.textContent());
  for (const token of fixture.required_text ?? []) assert(text.includes(token), `Calculate panel is missing '${token}': ${text}`);
}

async function executeArchiveRoute(page, fixture) {
  const archive = await requireFixtureFile(fixture, "archive");
  await page.evaluate((target) => window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path: target } })), archive);
  await page.locator('.nd-app[data-native-desktop-shell="true"]').waitFor({ state: "visible", timeout: 15_000 });
  const dialog = await selectMethod(page, fixture.method_kind ?? "cbsem");
  for (const selector of fixture.required_selectors ?? []) await dialog.locator(selector).waitFor({ state: "visible", timeout: 10_000 });
}

async function executeNamedDialog(page, fixture) {
  await loadSmokeFixture(page, fixture);
  assert(fixture.trigger?.selector || fixture.trigger?.key, "Fixture requires a dialog trigger selector or key.");
  if (fixture.trigger.selector) await page.locator(fixture.trigger.selector).click(); else await page.keyboard.press(fixture.trigger.key);
  const dialog = page.getByRole("dialog", { name: fixture.dialog_name, exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  if (fixture.expect_escape !== false) {
    await page.keyboard.press("Escape");
    await dialog.waitFor({ state: "hidden", timeout: 10_000 });
  }
}

async function executeDirtyGuard(page, fixture) {
  await loadSmokeFixture(page, fixture);
  assert(fixture.dirty_trigger?.selector && fixture.replace_trigger?.selector, "Fixture requires dirty and replacement triggers.");
  await page.locator(fixture.dirty_trigger.selector).click();
  await page.locator(fixture.replace_trigger.selector).click();
  const dialog = page.getByRole("dialog", { name: fixture.dialog_name, exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("button", { name: fixture.cancel_label ?? "Cancel", exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 10_000 });
}

async function executeOutcomeProfile(page, fixture) {
  await loadSmokeFixture(page, fixture);
  const dialog = await selectMethod(page, "regression");
  const outcome = dialog.locator(fixture.outcome_selector ?? "#nd-calculation-regression-outcome");
  await outcome.waitFor({ state: "visible", timeout: 10_000 });
  await outcome.selectOption(fixture.outcome_value);
  const blocker = dialog.locator(".nd-blocker");
  await blocker.waitFor({ state: "visible", timeout: 10_000 });
  for (const token of fixture.required_text ?? []) assert(compact(await blocker.textContent()).includes(token), `Outcome blocker lacks '${token}'.`);
}

async function executeInteractionScope(page, fixture) {
  await executeArchiveRoute(page, { ...fixture, method_kind: fixture.method_kind ?? "pls_algorithm" });
  const text = compact(await (await ensureCalculate(page)).textContent());
  for (const token of fixture.required_text ?? []) assert(text.includes(token), `Interaction scope display lacks '${token}'.`);
}

const EXECUTORS = {
  canvas_enter_edit: executeCanvasEnterEdit,
  calculate_registry: executeCalculateRegistry,
  archive_route: executeArchiveRoute,
  named_dialog: executeNamedDialog,
  dirty_guard: executeDirtyGuard,
  outcome_profile: executeOutcomeProfile,
  interaction_scope: executeInteractionScope,
};

async function executeCase(page, item, fixture, evidenceDir) {
  const base = {
    id: item.id,
    route: item.interaction_route,
    contract: item.replacement_contract,
    evidence_type: item.evidence_type,
    replacement_file: item.replacement_file ?? null,
    replacement_test: item.replacement_test ?? null,
    executed: false,
    status: "pending",
    fixture_status: fixture?.status ?? "missing",
    started_at: new Date().toISOString(),
  };
  if (item.evidence_type !== "browser") return { ...base, status: "failed", reason: "Only browser-evidence rows may be executed by a controlled browser fixture.", completed_at: new Date().toISOString() };
  if (!fixture || fixture.status !== "ready") return { ...base, reason: fixture?.reason ?? "No ready controlled fixture is declared for this replacement interaction.", completed_at: new Date().toISOString() };
  const executor = EXECUTORS[fixture.executor];
  if (!executor) return { ...base, reason: `Unknown executor '${fixture.executor}'.`, completed_at: new Date().toISOString() };
  base.executed = true;
  try {
    await executor(page, fixture);
    return { ...base, status: "passed", screenshot: await capture(page, evidenceDir, item.id), completed_at: new Date().toISOString() };
  } catch (error) {
    return { ...base, status: "failed", reason: error instanceof Error ? error.message : String(error), screenshot: await capture(page, evidenceDir, `${item.id}-failure`).catch(() => null), completed_at: new Date().toISOString() };
  }
}

async function browserContracts({ evidenceDir, port, fixturePath, vitestReportPath }) {
  const rebaseline = await readJson(REBASELINE);
  const matrix = await readJson(MATRIX);
  const fixtures = await readJson(fixturePath);
  const checks = validateContract(rebaseline, matrix, fixtures);
  assert(Object.values(checks).every(Boolean), `Replacement contract manifest invalid: ${JSON.stringify(checks)}`);
  const resolvedEvidence = path.resolve(evidenceDir);
  assert(inside(RESULTS, resolvedEvidence), "--evidence-dir must remain below validation/results.");
  const resolvedVitestReport = path.resolve(vitestReportPath);
  assert(inside(RESULTS, resolvedVitestReport), "--vitest-report must remain below validation/results.");
  const vitestReport = await readJson(resolvedVitestReport);
  const assertions = vitestAssertions(vitestReport);
  await fs.mkdir(resolvedEvidence, { recursive: true });
  const browserRows = rebaseline.items.filter((item) => item.evidence_type === "browser");
  const preview = browserRows.length ? startPreview(port) : null;
  let browser;
  const result = {
    schema_version: 3,
    suite_id: "quickpls_v255_rebased_interaction_contracts_v3",
    passed: false,
    checks,
    vitest_report: path.relative(ROOT, resolvedVitestReport).split(path.sep).join("/"),
    vitest_report_sha256: await sha256File(resolvedVitestReport),
    cases: [],
    failures: [],
  };
  try {
    const byId = new Map((fixtures.cases ?? []).map((fixture) => [fixture.id, fixture]));
    for (const item of rebaseline.items.filter((entry) => TEST_EVIDENCE_TYPES.has(entry.evidence_type))) {
      result.cases.push(executeTestBackedCase(item, assertions));
    }
    const errors = [];
    if (browserRows.length) {
      const origin = `http://127.0.0.1:${port}`;
      await waitForPreview(origin, preview.logs);
      browser = await chromium.launch({ headless: true });
      const page = await browser.newPage({ viewport: { width: 1024, height: 700 }, deviceScaleFactor: 1 });
      page.on("pageerror", (error) => errors.push(error.message));
      page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
      await page.goto(`${origin}/?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
      for (const item of browserRows) result.cases.push(await executeCase(page, item, byId.get(item.id), resolvedEvidence));
    }
    result.console_errors = errors;
    result.failures = result.cases.filter((entry) => entry.status !== "passed").map((entry) => `${entry.id}: ${entry.status}: ${entry.reason ?? "no successful execution"}`);
    if (errors.length) result.failures.push(`console: ${JSON.stringify(errors)}`);
    result.passed = result.failures.length === 0 && result.cases.length === 17;
  } finally {
    await browser?.close().catch(() => undefined);
    if (preview) stopPreview(preview.server, port);
    await fs.writeFile(path.join(resolvedEvidence, "v255_rebased_interaction_contracts.json"), `${JSON.stringify(result, null, 2)}\n`, "utf8");
  }
  if (!result.passed) throw new Error(result.failures[0] ?? "Interaction contract harness failed.");
  return result;
}

const args = parseArgs(process.argv.slice(2));
const rebaseline = await readJson(REBASELINE);
const matrix = await readJson(MATRIX);
const fixturePath = path.resolve(args["fixture-manifest"] ?? DEFAULT_FIXTURES);
const fixtureManifest = await readJson(fixturePath);
if (args.mode === "contract") {
  const checks = validateContract(rebaseline, matrix, fixtureManifest);
  const failed = Object.entries(checks).filter(([, passed]) => !passed).map(([id]) => id);
  console.log(JSON.stringify({ passed: failed.length === 0, checks, failed, fixture_manifest: path.relative(ROOT, fixturePath) }, null, 2));
  if (failed.length) process.exit(1);
} else {
  const port = Number(args.port ?? 57655);
  assert(Number.isInteger(port) && port > 0 && port < 65536, "--port must be a valid TCP port.");
  console.log(JSON.stringify(await browserContracts({ evidenceDir: args["evidence-dir"], port, fixturePath, vitestReportPath: args["vitest-report"] }), null, 2));
}
