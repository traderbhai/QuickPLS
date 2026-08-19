#!/usr/bin/env node
/** Product-bound Rank 0 General SEM performance driver.
 *
 * `prepare` authors the exact workload through the packaged QuickPLS UI and
 * saves a schema-6 General SEM project outside the timed boundary. `measure`
 * launches that exact package, reopens the prepared project, runs one native
 * calculation, emits real UI progress, and writes the harness result. `observe`
 * uses the same package/project to measure cancellation and the applied-profile
 * ten-run settled-memory soak. No scientific result is synthesized here.
 */

import fs from "node:fs/promises";
import path from "node:path";
import net from "node:net";
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import {
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
  structuralPaths,
  waitForSurface,
} from "./general_sem_rank0_packaged_acceptance.mjs";
import {
  connectToSingleQuickPlsPage,
  observeFunctionalOfflineRequests,
} from "./v247_cdp_package_helpers.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS_ROOT = path.join(ROOT, "validation", "results");
const COMMANDS = new Set(["prepare", "measure", "observe"]);
const SHA256 = /^[0-9a-f]{64}$/;
const EXACT_CASE_DIMENSIONS = Object.freeze({
  micro_exact: Object.freeze({ rows: 80, indicators: 15, constructs: 5 }),
  applied: Object.freeze({ rows: 1000, indicators: 40, constructs: 10 }),
  large: Object.freeze({ rows: 10000, indicators: 80, constructs: 20 }),
  maximum_rows_100000: Object.freeze({ rows: 100000, indicators: 40, constructs: 10 }),
  maximum_indicators_300: Object.freeze({ rows: 2000, indicators: 300, constructs: 20 }),
  maximum_constructs_100: Object.freeze({ rows: 5000, indicators: 300, constructs: 100 }),
  maximum_resamples_10000: Object.freeze({ rows: 1000, indicators: 40, constructs: 10 }),
  compound_stress: Object.freeze({ rows: 25000, indicators: 150, constructs: 50 }),
});

function parseArgs(argv) {
  const command = argv[0];
  if (!COMMANDS.has(command)) throw new Error("First argument must be prepare, measure, or observe.");
  const values = { command };
  for (let index = 1; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) throw new Error(`Unexpected positional argument: ${token}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${token}`);
    values[token.slice(2)] = value;
    index += 1;
  }
  const required = [
    "variant-id", "profile-id", "case-id", "workload-json",
    "quickpls-executable", "python-executable", "prepared-project", "driver-sha256",
  ];
  if (command === "observe") required.push("output", "accepted-runs");
  for (const key of required) if (!values[key]) throw new Error(`--${key} is required`);
  if (!VARIANTS[values["variant-id"]]) throw new Error(`Unknown Rank 0 variant: ${values["variant-id"]}`);
  if (!SHA256.test(values["driver-sha256"])) throw new Error("--driver-sha256 must be a lowercase SHA-256.");
  return values;
}

function sha256(bytes) { return createHash("sha256").update(bytes).digest("hex"); }
function compact(value) { return String(value ?? "").replace(/\s+/g, " ").trim(); }
function delay(milliseconds) { return new Promise((resolve) => setTimeout(resolve, milliseconds)); }
function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, stableValue(value[key])]));
  }
  return value;
}
function canonicalSha256(value) { return sha256(Buffer.from(JSON.stringify(stableValue(value)), "utf8")); }
function qualificationContractProjection(specification) {
  const evidence = specification?.evidence_contract;
  if (!evidence || typeof evidence !== "object") throw new Error("QualificationSpec evidence contract is unavailable.");
  return {
    schema_version: specification.schema_version,
    identity: specification.identity,
    scientific_contract: specification.scientific_contract,
    scenario_contract: specification.scenario_contract,
    comparison_contract: specification.comparison_contract,
    operational_contract: specification.operational_contract,
    evidence_contract: {
      required_roles: evidence.required_roles,
      receipt_contract: evidence.receipt_contract,
    },
  };
}
function workloadFingerprintAuthority(document) {
  const authority = { ...document };
  delete authority.workload_fingerprint;
  return authority;
}

function exactKeys(value, expected, subject) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`${subject} must be an object.`);
  const actual = Object.keys(value).sort();
  const frozen = [...expected].sort();
  if (JSON.stringify(actual) !== JSON.stringify(frozen)) {
    throw new Error(`${subject} fields are not exact: ${JSON.stringify({ expected: frozen, actual })}`);
  }
  return value;
}

function resolveInsideResults(value, subject, extension = null) {
  const resolved = path.resolve(value);
  const relative = path.relative(RESULTS_ROOT, resolved);
  if (!relative || relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`${subject} must remain below validation/results: ${resolved}`);
  }
  if (extension && path.extname(resolved).toLowerCase() !== extension) {
    throw new Error(`${subject} must use ${extension}: ${resolved}`);
  }
  return resolved;
}

async function writeJsonNew(file, value) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  const handle = await fs.open(file, "wx");
  try { await handle.writeFile(`${JSON.stringify(value, null, 2)}\n`, "utf8"); }
  finally { await handle.close(); }
}

async function loadWorkload(args) {
  const workloadPath = resolveInsideResults(args["workload-json"], "workload JSON", ".json");
  const value = JSON.parse(await fs.readFile(workloadPath, "utf8"));
  exactKeys(value, [
    "schema_version", "workload_kind", "variant_id", "capability_reference",
    "qualification_spec", "profile_id", "case_id", "mandatory_combination_id",
    "workload", "workload_fingerprint",
  ], "performance workload document");
  exactKeys(value.capability_reference, [
    "registry_schema_version", "capability_id", "cell_id", "capability_version",
  ], "workload capability reference");
  exactKeys(value.qualification_spec, [
    "path", "qualification_id", "spec_frozen_at_utc", "qualification_contract_sha256",
  ], "workload QualificationSpec authority");
  const workload = exactKeys(value.workload, [
    "rows", "indicators", "constructs", "resamples", "groups", "candidate_models",
  ], "performance workload");
  if (value.schema_version !== 1
    || value.workload_kind !== "general_sem_rank0_performance_workload"
    || value.variant_id !== args["variant-id"]
    || value.profile_id !== args["profile-id"]
    || value.case_id !== args["case-id"]
    || value.capability_reference.registry_schema_version !== 2
    || value.qualification_spec.spec_frozen_at_utc !== "2026-08-19T00:00:00Z"
    || !SHA256.test(value.qualification_spec.qualification_contract_sha256)
    || !SHA256.test(value.workload_fingerprint)
    || typeof value.mandatory_combination_id !== "string"
    || !value.mandatory_combination_id) {
    throw new Error("Performance workload identity does not match the exact driver request.");
  }
  const recordedFingerprint = value.workload_fingerprint;
  if (canonicalSha256(workloadFingerprintAuthority(value)) !== recordedFingerprint) {
    throw new Error("Performance workload fingerprint does not bind the exact authority document.");
  }
  const specPath = path.resolve(ROOT, value.qualification_spec.path);
  const specRelative = path.relative(path.join(ROOT, "validation", "qualification_v2"), specPath);
  if (!specRelative || specRelative === ".." || specRelative.startsWith(`..${path.sep}`) || path.isAbsolute(specRelative)) {
    throw new Error("Workload QualificationSpec path leaves validation/qualification_v2.");
  }
  const specBytes = await fs.readFile(specPath);
  const spec = JSON.parse(specBytes.toString("utf8"));
  if (canonicalSha256(qualificationContractProjection(spec)) !== value.qualification_spec.qualification_contract_sha256) {
    throw new Error("Workload normalized QualificationSpec contract digest differs.");
  }
  const combinations = spec?.scenario_contract?.mandatory_combinations;
  const combination = Array.isArray(combinations)
    ? combinations.filter((row) => row?.id === value.mandatory_combination_id)
    : [];
  if (spec?.identity?.qualification_id !== value.qualification_spec.qualification_id
    || JSON.stringify(stableValue(spec?.identity?.capability_cell)) !== JSON.stringify(stableValue(value.capability_reference))
    || combination.length !== 1
    || combination[0]?.profile_id !== value.profile_id) {
    throw new Error("Workload does not reconcile to the exact QualificationSpec identity/combination.");
  }
  for (const field of ["rows", "indicators", "constructs", "resamples", "groups", "candidate_models"]) {
    if (!Number.isSafeInteger(workload[field]) || workload[field] < (field === "resamples" ? 0 : 1)) {
      throw new Error(`workload.${field} is outside the frozen integer domain.`);
    }
  }
  if (workload.constructs < 4 || workload.indicators < workload.constructs * 2) {
    throw new Error("General SEM performance workload requires at least four constructs and two indicators per construct.");
  }
  const variant = VARIANTS[value.variant_id];
  if (value.case_id === "maximum_resamples_10000" && !variant.bootstrap) {
    throw new Error("The maximum-resamples case is inapplicable to a point-estimate cell.");
  }
  const expectedResamples = !variant.bootstrap
    ? 0
    : value.profile_id === "micro_exact"
      ? 199
      : ["applied", "large"].includes(value.profile_id)
        ? 5000
        : 10000;
  const exactDimensions = EXACT_CASE_DIMENSIONS[value.case_id];
  if (!exactDimensions) throw new Error(`Unknown frozen performance case: ${value.case_id}`);
  const expectedWorkload = {
    rows: exactDimensions.rows,
    indicators: exactDimensions.indicators,
    constructs: exactDimensions.constructs,
    resamples: expectedResamples,
    groups: 1,
    candidate_models: 1,
  };
  if (JSON.stringify(stableValue(workload)) !== JSON.stringify(stableValue(expectedWorkload))) {
    throw new Error("Driver workload is not the frozen single-group/single-candidate QualificationSpec workload.");
  }
  const profileRows = spec?.scenario_contract?.complexity_profiles;
  const profileMatches = Array.isArray(profileRows)
    ? profileRows.filter((row) => row?.id === value.profile_id && row?.applicability === "required")
    : [];
  if (profileMatches.length !== 1) throw new Error("QualificationSpec complexity profile is not exact and required.");
  const specWorkload = profileMatches[0]?.workload;
  const stressedField = {
    maximum_rows_100000: "rows",
    maximum_indicators_300: "indicators",
    maximum_constructs_100: "constructs",
    maximum_resamples_10000: "resamples",
  }[value.case_id];
  if (stressedField) {
    if (specWorkload?.[stressedField] !== workload[stressedField]
      || JSON.stringify(combination[0]?.stressed_dimensions) !== JSON.stringify([stressedField])) {
      throw new Error("Maximum-axis workload does not reach its frozen QualificationSpec dimension.");
    }
  } else if (JSON.stringify(stableValue(specWorkload)) !== JSON.stringify(stableValue(expectedWorkload))) {
    throw new Error("Profile workload differs from the exact QualificationSpec workload.");
  }
  return { document: value, workloadPath, workload, variant };
}

function constructDefinitions(variant, workload) {
  const firstNames = variant.family === "mediation" ? ["X", "M1", "M2", "Y"] : ["X", "W", "Z", "Y"];
  const names = Array.from({ length: workload.constructs }, (_, index) => (
    index < firstNames.length ? firstNames[index] : `C${String(index + 1).padStart(3, "0")}`
  ));
  const columns = Array.from({ length: workload.indicators }, (_, index) => `v${String(index + 1).padStart(3, "0")}`);
  const minimum = Math.floor(workload.indicators / workload.constructs);
  const remainder = workload.indicators % workload.constructs;
  let offset = 0;
  return names.map((name, index) => {
    const count = minimum + (index < remainder ? 1 : 0);
    const indicators = columns.slice(offset, offset + count);
    offset += count;
    return { name, indicators };
  });
}

async function writeDeterministicCsv(file, workload, definitions) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  const handle = await fs.open(file, "wx");
  const stream = handle.createWriteStream({ encoding: "utf8", autoClose: false });
  const constructByColumn = [];
  definitions.forEach((definition, construct) => {
    definition.indicators.forEach(() => constructByColumn.push(construct));
  });
  const columns = definitions.flatMap(({ indicators }) => indicators);
  stream.write(`${columns.join(",")}\n`);
  try {
    for (let row = 0; row < workload.rows; row += 1) {
      const values = columns.map((_, column) => {
        const construct = constructByColumn[column];
        const common = Math.sin((row + 1) * (construct + 3) * 0.017)
          + Math.cos((row + 7) * (construct + 1) * 0.011);
        const unique = Math.sin((row + 5) * (column + 11) * 0.0037)
          + Math.cos((row + 13) * (column + 2) * 0.0051);
        const directed = construct === 3
          ? 0.22 * Math.sin((row + 1) * 0.019) + 0.12 * Math.cos((row + 1) * 0.027)
          : 0;
        return (common + unique * 0.18 + directed).toFixed(8);
      });
      if (!stream.write(`${values.join(",")}\n`)) await once(stream, "drain");
    }
    stream.end();
    await once(stream, "finish");
    await handle.sync();
  } finally {
    await handle.close();
  }
  const stat = await fs.stat(file);
  if (stat.size <= 0) throw new Error("Prepared performance CSV is empty.");
  return { path: file, size: stat.size, sha256: sha256(await fs.readFile(file)) };
}

async function findFreePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("Could not reserve a private CDP port.");
  const port = address.port;
  await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  return port;
}

async function waitCdp(endpoint, child) {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (await fetch(`${endpoint}/json/version`).then((response) => response.ok).catch(() => false)) return;
    if (child.exitCode !== null) throw new Error(`Packaged QuickPLS exited before CDP became ready (${child.exitCode}).`);
    await delay(250);
  }
  throw new Error(`Packaged QuickPLS CDP did not become ready at ${endpoint}.`);
}

async function powershellJson(script, args = []) {
  const child = spawn("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script, ...args.map(String)], {
    cwd: ROOT, windowsHide: true, stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8"); child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.setEncoding("utf8"); child.stderr.on("data", (chunk) => { stderr += chunk; });
  const [code] = await once(child, "close");
  if (code !== 0) throw new Error(`PowerShell process observation failed: ${stderr || stdout}`);
  return JSON.parse(stdout.trim() || "null");
}

async function processTreeSnapshot(rootPid) {
  const script = [
    "$rootPidValue = [int]$args[0]",
    "$rows = @(Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object ProcessId,ParentProcessId,CreationDate,Name,WorkingSetSize)",
    "$ids = [System.Collections.Generic.HashSet[int]]::new()",
    "$pending = [System.Collections.Generic.Queue[int]]::new()",
    "$null = $ids.Add($rootPidValue); $pending.Enqueue($rootPidValue)",
    "while ($pending.Count -gt 0) { $parent = $pending.Dequeue(); foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parent }) { $id = [int]$row.ProcessId; if ($ids.Add($id)) { $pending.Enqueue($id) } } }",
    "$selected = @($rows | Where-Object { $ids.Contains([int]$_.ProcessId) })",
    "$result = [ordered]@{ pids = @($selected | ForEach-Object { [int]$_.ProcessId }); workingSetBytes = [long](($selected | Measure-Object -Property WorkingSetSize -Sum).Sum) }",
    "$result | ConvertTo-Json -Compress",
  ].join("; ");
  const value = await powershellJson(script, [rootPid]);
  if (!value || !Array.isArray(value.pids) || !Number.isSafeInteger(value.workingSetBytes) || value.workingSetBytes <= 0) {
    throw new Error(`Could not observe the exact QuickPLS process tree: ${JSON.stringify(value)}`);
  }
  return value;
}

async function liveProcessCount(pids) {
  if (!pids.length) return 0;
  const script = [
    "$count = 0",
    "foreach ($pidValue in $args) { if (Get-Process -Id ([int]$pidValue) -ErrorAction SilentlyContinue) { $count++ } }",
    "$count | ConvertTo-Json -Compress",
  ].join("; ");
  const value = await powershellJson(script, pids);
  return Number(value);
}

async function launchPackage(executable) {
  const observedExecutable = path.resolve(executable);
  const stat = await fs.stat(observedExecutable);
  if (!stat.isFile() || path.extname(observedExecutable).toLowerCase() !== ".exe") {
    throw new Error(`Packaged QuickPLS executable is invalid: ${observedExecutable}`);
  }
  const port = await findFreePort();
  const endpoint = `http://127.0.0.1:${port}`;
  const browserArguments = `--remote-debugging-port=${port} --force-device-scale-factor=1 --disable-background-networking --disable-component-update`;
  const child = spawn(observedExecutable, [], {
    cwd: path.dirname(observedExecutable),
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: browserArguments,
      QUICKPLS_CDP_ENDPOINT: endpoint,
    },
    windowsHide: false,
    stdio: "ignore",
  });
  child.on("error", () => undefined);
  await waitCdp(endpoint, child);
  const connected = await connectToSingleQuickPlsPage({ chromium, endpoint });
  const { browser, page } = connected;
  await page.evaluate(() => localStorage.setItem(
    "quickpls:native-ui-preferences:v1",
    JSON.stringify({ experimentalLabsEnabled: false }),
  ));
  await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  return { child, endpoint, browser, page, forcedCleanup: false };
}

async function closePackage(session) {
  let descriptors = { pids: [session.child.pid], workingSetBytes: 1 };
  try { descriptors = await processTreeSnapshot(session.child.pid); } catch { /* Exact close checks below still fail closed. */ }
  let invokeError = null;
  try {
    await session.page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke("exit_desktop_application");
    });
  } catch (error) {
    invokeError = error;
  }
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const closed = !await fetch(`${session.endpoint}/json/version`).then((response) => response.ok).catch(() => false);
    const live = await liveProcessCount(descriptors.pids);
    if (closed && live === 0) {
      await session.browser.close().catch(() => undefined);
      return { orphanProcesses: 0, cdpClosed: true, forcedCleanup: false };
    }
    await delay(100);
  }
  session.forcedCleanup = true;
  await session.browser.close().catch(() => undefined);
  if (session.child.exitCode === null) session.child.kill();
  throw new Error(`Packaged QuickPLS did not close cleanly: ${String(invokeError ?? "live process or CDP endpoint remained")}`);
}

async function buildPerformanceModel(page, variant, definitions) {
  const nodes = await buildConstructs(page, definitions);
  let expectedPaths = 0;
  if (variant.family === "mediation") {
    for (const [source, target] of [[0, 1], [0, 2], [1, 3], [2, 3], [0, 3]]) {
      expectedPaths += 1;
      await createPath(page, nodes, source, target, expectedPaths);
    }
  } else {
    expectedPaths += 1;
    await createPath(page, nodes, 0, 3, expectedPaths);
  }
  for (let index = 4; index < definitions.length; index += 1) {
    expectedPaths += 1;
    await createPath(page, nodes, 0, index, expectedPaths);
    expectedPaths += 1;
    await createPath(page, nodes, index, 3, expectedPaths);
  }
  if (variant.family === "moderation") {
    const baseId = await structuralPaths(page).first().getAttribute("data-id");
    if (!baseId) throw new Error("Prepared X -> Y path has no stable identity.");
    const base = page.locator(`.react-flow__edge[data-id="${baseId}"]`);
    await addModerator(page, base, "W");
    await addModerator(page, base, "Z");
  }
  return {
    constructs: definitions.length,
    indicators: definitions.reduce((sum, definition) => sum + definition.indicators.length, 0),
    structuralPaths: expectedPaths,
    interactions: variant.family === "moderation" ? 2 : 0,
  };
}

function calculationButton(page, bootstrap) {
  return page.getByRole("button", {
    name: bootstrap ? /Calculate .*bootstrap|Calculate PLS effects/i : /Calculate .*point estimates|Calculate PLS effects/i,
  });
}

async function emitProgress(value) {
  const progressPath = process.env.QPLS_PERFORMANCE_PROGRESS_PATH;
  if (!progressPath) return;
  await fs.appendFile(progressPath, `${JSON.stringify({ progress: value, observed_at_utc: new Date().toISOString() })}\n`, "utf8");
}

async function runCalculation(page, bootstrap, writeProgress) {
  const workspace = page.locator("#nd-model-general-sem-labs-panel");
  const start = calculationButton(page, bootstrap);
  if (!await start.isEnabled()) throw new Error("Prepared native calculation is not enabled.");
  if (writeProgress) await emitProgress(0);
  const started = performance.now();
  await start.click();
  const monitor = workspace.locator(".nd-cbsem-v4-monitor");
  await monitor.waitFor({ state: "visible", timeout: 20_000 });
  const progress = monitor.locator("progress");
  if (await monitor.count() !== 1 || await progress.count() !== 1) {
    throw new Error("The exact General SEM progress monitor is missing or ambiguous.");
  }
  let last = 0;
  const deadline = Date.now() + 12 * 60 * 60 * 1000;
  while (Date.now() < deadline) {
    const state = compact(await monitor.locator(".nd-cbsem-v4-state").textContent()).toLowerCase();
    const value = Number(await progress.getAttribute("value") ?? 0);
    const maximum = Math.max(1, Number(await progress.getAttribute("max") ?? 1));
    const fraction = Math.max(0, Math.min(1, value / maximum));
    if (fraction < last) throw new Error("Native calculation progress regressed.");
    if (fraction > last) {
      last = fraction;
      if (writeProgress) await emitProgress(fraction);
    }
    if (state === "failed" || state === "cancelled") throw new Error(`Native calculation terminated as ${state}.`);
    if (state === "completed") break;
    await page.waitForTimeout(100);
  }
  if (Date.now() >= deadline) throw new Error("Native calculation exceeded the 12-hour driver timeout.");
  const result = workspace.locator(".nd-cbsem-v4-results");
  await result.waitFor({ state: "visible", timeout: 30_000 });
  if (await result.count() !== 1) throw new Error("The exact General SEM result surface is missing or ambiguous.");
  await result.locator("#nd-cbsem-v4-results-heading").filter({ hasText: /General SEM.*result/i }).waitFor({ state: "visible", timeout: 10_000 });
  if (await result.locator("[data-canonical-table-id]").count() < 1) {
    throw new Error("The General SEM result surface has no canonical result table.");
  }
  if (last < 1 && writeProgress) await emitProgress(1);
  return { elapsedSeconds: (performance.now() - started) / 1000, identity: await canonicalIdentity(page) };
}

async function dismissTemporaryResult(page, bootstrap) {
  const dismiss = page.getByRole("button", { name: "Dismiss temporary result", exact: true });
  await dismiss.waitFor({ state: "visible", timeout: 10_000 });
  await dismiss.click();
  await dismiss.waitFor({ state: "hidden", timeout: 10_000 });
  const start = calculationButton(page, bootstrap);
  await start.waitFor({ state: "visible", timeout: 10_000 });
  if (!await start.isEnabled()) throw new Error("Calculation did not become retryable after dismissing the temporary result.");
}

async function prepare(args, loaded) {
  const preparedProject = resolveInsideResults(args["prepared-project"], "prepared project", ".qpls");
  await fs.stat(preparedProject).then(() => { throw new Error(`Prepared project must be new: ${preparedProject}`); }).catch((error) => {
    if (error?.code !== "ENOENT") throw error;
  });
  const definitions = constructDefinitions(loaded.variant, loaded.workload);
  const fixturePath = preparedProject.replace(/\.qpls$/i, ".input.csv");
  const fixture = await writeDeterministicCsv(fixturePath, loaded.workload, definitions);
  const session = await launchPackage(args["quickpls-executable"]);
  const offline = observeFunctionalOfflineRequests(session.page);
  let cleanup;
  try {
    await createGeneralSemDraft(session.page, `Rank 0 performance ${args["variant-id"]} ${args["case-id"]}`);
    await importFixture(session.page, args["python-executable"], fixturePath);
    await createEmptyModel(session.page, "Rank 0 performance model");
    await openGeneralSem(session.page);
    if (await session.page.getByRole("button", { name: "Save and activate project…", exact: true }).isEnabled()) {
      throw new Error("Empty performance model did not fail closed before authoring.");
    }
    await session.page.locator("#nd-model-canvas-tab").click();
    const model = await buildPerformanceModel(session.page, loaded.variant, definitions);
    await openGeneralSem(session.page);
    await configureInference(session.page, loaded.variant.bootstrap);
    if (loaded.variant.bootstrap) {
      await session.page.locator("#nd-general-sem-bootstrap-samples").fill(String(loaded.workload.resamples));
    }
    await saveAndActivate(session.page, args["python-executable"], preparedProject);
    const stat = await fs.stat(preparedProject);
    if (stat.size <= 0) throw new Error("Packaged preparation produced an empty project archive.");
    const offlineSummary = offline.summary();
    if (!offlineSummary.passed) throw new Error(`Preparation crossed the offline boundary: ${JSON.stringify(offlineSummary)}`);
    cleanup = await closePackage(session);
    console.log(JSON.stringify({
      passed: true,
      command: "prepare",
      prepared_project: preparedProject,
      prepared_project_size: stat.size,
      workload_sha256: sha256(await fs.readFile(loaded.workloadPath)),
      fixture,
      model,
      cleanup,
    }));
  } catch (error) {
    await closePackage(session).catch(() => undefined);
    throw error;
  } finally {
    offline.stop();
  }
}

async function openPreparedSession(args) {
  const preparedProject = resolveInsideResults(args["prepared-project"], "prepared project", ".qpls");
  const stat = await fs.stat(preparedProject);
  if (!stat.isFile() || stat.size <= 0) throw new Error("Prepared project is unavailable or empty.");
  const session = await launchPackage(args["quickpls-executable"]);
  const offline = observeFunctionalOfflineRequests(session.page);
  try {
    await openExactProject(session.page, preparedProject);
    return { session, offline, preparedProject };
  } catch (error) {
    offline.stop();
    await closePackage(session).catch(() => undefined);
    throw error;
  }
}

async function measure(args, loaded) {
  const resultPath = resolveInsideResults(process.env.QPLS_PERFORMANCE_RESULT_PATH ?? "", "performance result", ".json");
  const phase = process.env.QPLS_PERFORMANCE_PHASE;
  const runIndex = Number(process.env.QPLS_PERFORMANCE_RUN_INDEX ?? -1);
  if (!["warmup", "measured"].includes(phase) || !Number.isSafeInteger(runIndex) || runIndex < 0) {
    throw new Error("Performance phase/index authority is missing or invalid.");
  }
  const { session, offline, preparedProject } = await openPreparedSession(args);
  try {
    const run = await runCalculation(session.page, loaded.variant.bootstrap, true);
    const offlineSummary = offline.summary();
    if (!offlineSummary.passed) throw new Error(`Measured calculation crossed the offline boundary: ${JSON.stringify(offlineSummary)}`);
    const workloadBytes = await fs.readFile(loaded.workloadPath);
    const executableBytes = await fs.readFile(path.resolve(args["quickpls-executable"]));
    const payload = {
      schema_version: 1,
      evidence_kind: "general_sem_rank0_packaged_performance_result",
      variant_id: args["variant-id"],
      capability_reference: loaded.document.capability_reference,
      qualification_contract_sha256: loaded.document.qualification_spec.qualification_contract_sha256,
      profile_id: args["profile-id"],
      case_id: args["case-id"],
      mandatory_combination_id: loaded.document.mandatory_combination_id,
      workload_fingerprint: loaded.document.workload_fingerprint,
      workload_document_sha256: sha256(workloadBytes),
      performance_driver_sha256: args["driver-sha256"],
      prepared_project: preparedProject,
      package_executable_sha256: sha256(executableBytes),
      phase,
      run_index: runIndex,
      operation_elapsed_seconds: run.elapsedSeconds,
      canonical_identity: run.identity,
      offline: offlineSummary,
      completed: true,
    };
    await writeJsonNew(resultPath, payload);
    const archivePath = path.join(
      path.dirname(resultPath),
      `${path.basename(resultPath, ".json")}.payloads`,
      `${phase}-${runIndex}.json`,
    );
    await writeJsonNew(archivePath, payload);
    await dismissTemporaryResult(session.page, loaded.variant.bootstrap);
    const cleanup = await closePackage(session);
    console.log(JSON.stringify({ passed: true, command: "measure", result: resultPath, cleanup }));
  } catch (error) {
    await closePackage(session).catch(() => undefined);
    throw error;
  } finally {
    offline.stop();
  }
}

async function observe(args, loaded) {
  const output = resolveInsideResults(args.output, "performance observation", ".json");
  const acceptedRuns = Number(args["accepted-runs"]);
  if (!Number.isSafeInteger(acceptedRuns) || acceptedRuns !== 10) {
    throw new Error("Applied memory-soak observation requires exactly ten accepted runs.");
  }
  const { session, offline, preparedProject } = await openPreparedSession(args);
  const observations = {};
  try {
    const start = calculationButton(session.page, loaded.variant.bootstrap);
    const cancelled = await cancelAndVerify(
      session.page,
      start,
      args["python-executable"],
      preparedProject,
    );
    observations.cancellation_observation = {
      terminal_latency_seconds: cancelled.terminalLatencySeconds,
      terminal_state: cancelled.terminalState,
      no_partial_visible_result: cancelled.noPartialVisibleResult,
      no_partial_committed_result: cancelled.noPartialCommittedResult,
      archive_unchanged: cancelled.archiveUnchanged,
    };
    if (args["profile-id"] === "applied") {
      const settledWorkingSets = [];
      for (let index = 0; index < acceptedRuns; index += 1) {
        await runCalculation(session.page, loaded.variant.bootstrap, false);
        await dismissTemporaryResult(session.page, loaded.variant.bootstrap);
        await session.page.waitForTimeout(500);
        settledWorkingSets.push((await processTreeSnapshot(session.child.pid)).workingSetBytes);
      }
      const first = settledWorkingSets[0];
      const last = settledWorkingSets.at(-1);
      observations.memory_growth_observation = {
        accepted_runs: acceptedRuns,
        first_settled_working_set_bytes: first,
        last_settled_working_set_bytes: last,
        growth_percent: Math.max(0, ((last - first) / first) * 100),
        orphan_processes: 0,
      };
    }
    const offlineSummary = offline.summary();
    if (!offlineSummary.passed) throw new Error(`Performance observation crossed the offline boundary: ${JSON.stringify(offlineSummary)}`);
    const cleanup = await closePackage(session);
    if (cleanup.orphanProcesses !== 0 || !cleanup.cdpClosed || cleanup.forcedCleanup) {
      throw new Error(`Performance observer did not close exactly: ${JSON.stringify(cleanup)}`);
    }
    await writeJsonNew(output, observations);
    console.log(JSON.stringify({ passed: true, command: "observe", output, cleanup }));
  } catch (error) {
    await closePackage(session).catch(() => undefined);
    throw error;
  } finally {
    offline.stop();
  }
}

export {
  constructDefinitions,
  loadWorkload,
  parseArgs,
  resolveInsideResults,
};

export async function runPerformanceDriver(argv = process.argv.slice(2)) {
  if (process.platform !== "win32") throw new Error("Rank 0 packaged performance requires Windows.");
  const args = parseArgs(argv);
  const ownSha256 = sha256(await fs.readFile(fileURLToPath(import.meta.url)));
  if (ownSha256 !== args["driver-sha256"]) {
    throw new Error("Bundled performance driver bytes differ from the orchestrator-bound SHA-256.");
  }
  const loaded = await loadWorkload(args);
  if (args.command === "prepare") await prepare(args, loaded);
  else if (args.command === "measure") await measure(args, loaded);
  else await observe(args, loaded);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { await runPerformanceDriver(); }
  catch (error) {
    console.error(JSON.stringify({ passed: false, error: error instanceof Error ? error.message : String(error) }));
    process.exitCode = 1;
  }
}
