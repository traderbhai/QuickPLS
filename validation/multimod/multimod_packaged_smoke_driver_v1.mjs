#!/usr/bin/env node
/* Exact packaged-candidate qualification over the wrapper-owned WebView2 CDP session. */

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { chromium } from "playwright";
import {
  PACKAGED_TAURI_ORIGIN,
  connectToSingleQuickPlsPage,
} from "../v247_cdp_package_helpers.mjs";

const TERMINAL = new Set(["completed", "failed", "cancelled"]);
const STANDARD_MULTIMOD_SURFACE = "standard_multimod_v1";
const FAMILY_EXPECTATIONS = Object.freeze({
  "qpls.multimod.mga_multigroup_v1": Object.freeze({ target: "mga_multigroup_v1", resultKind: "pls_multigroup_analysis_v1", maximumSeconds: 7200 }),
  "qpls.multimod.pls_heterogeneity_v2": Object.freeze({ target: "pls_heterogeneity_v2", resultKind: "pls_heterogeneity_analysis_v2", maximumSeconds: 7200 }),
  "qpls.multimod.general_sem_conditional_process_v2": Object.freeze({ target: "general_sem_conditional_process_v2", resultKind: "general_sem_conditional_process_result_v2", maximumSeconds: 3600 }),
  "qpls.multimod.interventional_causal_mediation_v1": Object.freeze({ target: "interventional_causal_mediation_v1", resultKind: "interventional_mediation_result_v1", maximumSeconds: 3600 }),
});

function parseArguments(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value === undefined) throw new Error("Arguments must be --name value pairs.");
    values[key.slice(2)] = value;
  }
  for (const required of [
    "endpoint", "kind", "candidate-path", "candidate-sha256", "candidate-pid",
    "candidate-commit", "candidate-version", "plan-sha256", "binding-sha256",
    "authority-document-sha256", "authority-binding-sha256",
    "prepackage-manifest-set-sha256", "package-receipt-sha256",
    "scientific-deadline-epoch-ms", "seed", "output",
  ]) {
    if (!values[required]) throw new Error(`--${required} is required.`);
  }
  if (!new Set(["installed", "portable"]).has(values.kind)) throw new Error("--kind must be installed or portable.");
  return values;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function remainingScientificMilliseconds(operation) {
  const remaining = scientificDeadlineEpochMs - Date.now();
  assert(remaining > 0, `${operation} reached the driver-wide scientific deadline.`);
  return remaining;
}

function assertWithinScientificDeadline(operation) {
  remainingScientificMilliseconds(operation);
}

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function hashFile(file) {
  return sha256(fs.readFileSync(file));
}

function writeJsonAtomic(file, value) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const temporary = `${file}.tmp`;
  fs.writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  fs.renameSync(temporary, file);
}

function stableSlug(value) {
  return value.replace(/^qpls\.multimod\./u, "").replace(/[^a-z0-9._-]+/giu, "-").toLowerCase();
}

async function nativeInvoke(page, command, payload = {}) {
  assertWithinScientificDeadline(`Native command ${command} start`);
  const outcome = await page.evaluate(async ({ command: commandName, payload: invokePayload }) => {
    const invoke = globalThis.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== "function") throw new Error("Tauri invoke bridge is unavailable.");
    try {
      return { ok: true, value: await invoke(commandName, invokePayload) };
    } catch (error) {
      return { ok: false, error, message: String(error) };
    }
  }, { command, payload });
  if (!outcome.ok) {
    throw new Error(`Native command ${command} rejected: ${JSON.stringify({ error: outcome.error, message: outcome.message })}`);
  }
  assertWithinScientificDeadline(`Native command ${command} completion`);
  return outcome.value;
}

async function expectInvokeRejected(page, command, payload) {
  assertWithinScientificDeadline(`Rejected native command ${command} start`);
  const value = await page.evaluate(async ({ command: commandName, payload: invokePayload }) => {
    try {
      await globalThis.__TAURI_INTERNALS__.invoke(commandName, invokePayload);
      return { rejected: false, message: "" };
    } catch (error) {
      return { rejected: true, message: String(error) };
    }
  }, { command, payload });
  assertWithinScientificDeadline(`Rejected native command ${command} completion`);
  return value;
}

async function pollJob(page, jobId, maximumSeconds, predicate = (snapshot) => TERMINAL.has(snapshot.state)) {
  const familyDeadlineEpochMs = Date.now() + maximumSeconds * 1000;
  const deadline = Math.min(familyDeadlineEpochMs, scientificDeadlineEpochMs);
  assert(deadline > Date.now(), `MultiMod job ${jobId} has no driver-wide scientific time remaining.`);
  let snapshot;
  while (Date.now() < deadline) {
    snapshot = await nativeInvoke(page, "status_internal_labs_multimod_job_v1", { jobId });
    if (predicate(snapshot)) {
      assert(Date.now() <= deadline, `MultiMod job ${jobId} reached a terminal predicate after its effective deadline.`);
      return snapshot;
    }
    const remaining = deadline - Date.now();
    if (remaining > 0) await new Promise((resolve) => setTimeout(resolve, Math.min(100, remaining)));
  }
  throw new Error(`MultiMod job ${jobId} exceeded the earlier of its ${maximumSeconds}-second family budget and the driver-wide scientific deadline; last state was ${snapshot?.state ?? "unknown"}.`);
}

function stagedRequest(fixture) {
  const expected = FAMILY_EXPECTATIONS[fixture.familyId];
  assert(expected, `Unknown fixture family ${fixture.familyId}.`);
  return {
    surface: STANDARD_MULTIMOD_SURFACE,
    experimentalLabsEnabled: false,
    archivePath: fixture.authority.archivePath,
    expectedArchiveSha256: fixture.authority.archiveSha256,
    projectId: fixture.authority.projectId,
    datasetId: fixture.authority.datasetId,
    datasetFingerprint: fixture.authority.datasetFingerprint,
    modelId: fixture.authority.modelId,
    modelScientificSha256: fixture.authority.modelScientificSha256,
    sourceRecipeId: fixture.authority.sourceRecipeId,
    sourceRecipeDocumentSha256: fixture.authority.sourceRecipeDocumentSha256,
    stagedRecipeId: crypto.randomUUID(),
    stagedCreatedAt: new Date().toISOString(),
    config: { kind: expected.target, config: fixture.config },
  };
}

function assertCandidateResult(result, fixture, expected, authority) {
  assert(result?.schemaVersion === 1, `${fixture.familyId} completed-result schema is invalid.`);
  assert(result.archivePath === fixture.authority.archivePath, `${fixture.familyId} published to a different archive.`);
  assert(result.projectId === fixture.authority.projectId, `${fixture.familyId} project identity changed.`);
  assert(result.datasetId === fixture.authority.datasetId, `${fixture.familyId} dataset identity changed.`);
  assert(result.modelId === fixture.authority.modelId, `${fixture.familyId} model identity changed.`);
  const attachment = result.attachment;
  assert(attachment?.result?.kind === expected.resultKind, `${fixture.familyId} returned the wrong typed result family.`);
  assert(attachment.result_id === result.appendReceipt?.result_id, `${fixture.familyId} result and append identities differ.`);
  assert(attachment.result_id === result.canonicalDocument?.provenance?.run_id, `${fixture.familyId} canonical run identity differs.`);
  assert(result.archiveSha256 === result.appendReceipt?.updated_archive_sha256, `${fixture.familyId} archive receipt digest differs.`);
  assert(result.cacheRemovedAfterCommit === true, `${fixture.familyId} left an external publication cache after commit.`);
  const provenance = attachment.result.analysis?.provenance;
  assert(provenance?.qualification === "release_qualified_candidate", `${fixture.familyId} was not promoted by embedded candidate authority.`);
  const receipt = provenance?.candidate_qualification_receipt;
  assert(receipt?.authority_binding_sha256 === authority.authorityBindingSha256, `${fixture.familyId} authority receipt binding differs.`);
  assert(receipt?.candidate_commit_sha === authority.candidateCommitSha, `${fixture.familyId} authority receipt commit differs.`);
  assert(receipt?.candidate_version === authority.candidateVersion, `${fixture.familyId} authority receipt version differs.`);
  assert(Array.isArray(receipt?.required_profile_cells) && receipt.required_profile_cells.length > 0, `${fixture.familyId} has no exact receipt cells.`);
  assert(receipt.required_profile_cells.every((cell) => authority.exactProfileCells.includes(cell)), `${fixture.familyId} receipt claims a cell outside embedded authority.`);
  assert(Array.isArray(attachment.sidecars) && attachment.sidecars.length > 0, `${fixture.familyId} published no strict evidence sidecars.`);
  assert(attachment.sidecars.every((sidecar) => sidecar.identity_sha256 === attachment.identity_sha256), `${fixture.familyId} sidecar identity differs from its attachment.`);
}

async function runFamily(page, fixture, authority) {
  const expected = FAMILY_EXPECTATIONS[fixture.familyId];
  const request = stagedRequest(fixture);
  const preflight = await nativeInvoke(page, "preflight_internal_labs_multimod_v1", { request });
  assert(preflight?.schemaVersion === 1 && preflight.target === expected.target, `${fixture.familyId} preflight identity is invalid.`);
  assert(preflight.readiness === "built_in_from_dataset", `${fixture.familyId} is not executable through the production dataset adapter.`);

  if (fixture.cancellationRecoveryRequired) {
    const started = await nativeInvoke(page, "start_internal_labs_multimod_job_v1", { request });
    const cancellable = await pollJob(page, started.jobId, 120, (snapshot) => snapshot.resumeCache != null || TERMINAL.has(snapshot.state));
    assert(!TERMINAL.has(cancellable.state), "MGA completed before a durable resumable checkpoint could be cancelled.");
    assert(cancellable.resumeCache?.stage === "mga_execution", "MGA did not expose its production execution-cache checkpoint.");
    await nativeInvoke(page, "cancel_internal_labs_multimod_job_v1", { jobId: started.jobId });
    const cancelled = await pollJob(page, started.jobId, 120);
    assert(cancelled.state === "cancelled", `MGA cancellation ended as ${cancelled.state}.`);
    assert(cancelled.resumeCache?.cacheId === cancellable.resumeCache.cacheId, "MGA cancellation changed its validated cache identity.");
    const partialResult = await expectInvokeRejected(page, "result_internal_labs_multimod_job_v1", { jobId: started.jobId });
    assert(partialResult.rejected, "Cancelled MGA exposed a partial scientific result.");
    assert(hashFile(fixture.authority.archivePath) === fixture.authority.archiveSha256, "Cancelled MGA mutated the source archive.");
    request.resumeCache = cancelled.resumeCache;
    const resumePreflight = await nativeInvoke(page, "preflight_internal_labs_multimod_v1", { request });
    assert(resumePreflight.compilationIdentitySha256 === preflight.compilationIdentitySha256, "MGA resume changed compiled scientific identity.");
    const resumed = await nativeInvoke(page, "start_internal_labs_multimod_job_v1", { request });
    const terminal = await pollJob(page, resumed.jobId, expected.maximumSeconds);
    assert(terminal.state === "completed", `Resumed MGA ended as ${terminal.state}: ${JSON.stringify(terminal.failure ?? {})}`);
    const result = await nativeInvoke(page, "result_internal_labs_multimod_job_v1", { jobId: resumed.jobId });
    assertCandidateResult(result, fixture, expected, authority);
    return {
      result,
      preflight,
      cancellation: {
        cancelled_job_id: started.jobId,
        resumed_job_id: resumed.jobId,
        cache_id: cancelled.resumeCache.cacheId,
        cache_manifest_sha256: cancelled.resumeCache.manifestSha256,
        partial_result_suppressed: true,
        source_archive_unchanged_before_resume: true,
        deterministic_compilation_identity_preserved: true,
      },
    };
  }

  const started = await nativeInvoke(page, "start_internal_labs_multimod_job_v1", { request });
  const terminal = await pollJob(page, started.jobId, expected.maximumSeconds);
  assert(terminal.state === "completed", `${fixture.familyId} ended as ${terminal.state}: ${JSON.stringify(terminal.failure ?? {})}`);
  const result = await nativeInvoke(page, "result_internal_labs_multimod_job_v1", { jobId: started.jobId });
  assertCandidateResult(result, fixture, expected, authority);
  return { result, preflight };
}

async function exportCanonicalMatrix(page, document, directory, stem) {
  fs.mkdirSync(directory, { recursive: true });
  const receipts = await page.evaluate(async ({ document: canonical, directory: destination, stem: fileStem }) => {
    const bridge = globalThis.__QPLS_MULTIMOD_PACKAGED_QUALIFICATION_V1__;
    if (!bridge) throw new Error("The build-only packaged qualification bridge is unavailable.");
    return bridge.exportCanonicalMatrix(canonical, destination, fileStem);
  }, { document, directory, stem });
  const formats = new Set(receipts.map((receipt) => receipt.format));
  for (const format of ["csv", "xlsx", "json", "html", "pdf"]) assert(formats.has(format), `Canonical export matrix omitted ${format}.`);
  return receipts.map((receipt) => {
    const bytes = fs.readFileSync(receipt.path);
    assert(bytes.length > 0, `Canonical ${receipt.format} export is empty.`);
    assert(receipt.publication?.sha256 === sha256(bytes), `Canonical ${receipt.format} publication digest differs from disk.`);
    assert(receipt.exactSemanticReadback && receipt.digestReadback && receipt.renderedSurfaceReadback, `Canonical ${receipt.format} semantic readback was incomplete.`);
    return { ...receipt, size: bytes.length, sha256: sha256(bytes) };
  });
}

async function verifyTamperedCandidateReceiptFails(page, document, directory, stem) {
  fs.mkdirSync(directory, { recursive: true });
  const outcome = await page.evaluate(async ({ document: source, directory: destination, stem: fileStem }) => {
    const tampered = structuredClone(source);
    const table = tampered.tables.find((candidate) => candidate.id === "multimod_run_provenance");
    const column = table?.columns.findIndex((candidate) => candidate.id === "candidate_qualification_receipt_json") ?? -1;
    const row = table?.rows.find((candidate) => candidate.id === "run");
    const cell = column >= 0 ? row?.cells[column] : undefined;
    if (cell?.kind !== "text") throw new Error("Canonical candidate receipt cell is unavailable.");
    const receipt = JSON.parse(cell.value);
    receipt.qualification_plan_sha256 = "0".repeat(64);
    cell.value = JSON.stringify(receipt);
    try {
      await globalThis.__QPLS_MULTIMOD_PACKAGED_QUALIFICATION_V1__.exportCanonicalMatrix(tampered, destination, fileStem);
      return { rejected: false, message: "" };
    } catch (error) {
      return { rejected: true, message: String(error) };
    }
  }, { document, directory, stem });
  assert(outcome.rejected, "Candidate canonical export accepted a receipt that differed from embedded authority.");
  assert(fs.readdirSync(directory).length === 0, "Rejected candidate-receipt export published bytes.");
  return outcome;
}

async function inspectStrict(page, archivePath, expectedSha) {
  const outcome = await nativeInvoke(page, "inspect_internal_project_archive_v6_zip", {
    request: { surface: STANDARD_MULTIMOD_SURFACE, experimentalLabsEnabled: false, archivePath },
  });
  assert(outcome?.status === "ok", `Strict Archive V6 reopen failed: ${JSON.stringify(outcome?.diagnostic ?? {})}`);
  assert(outcome.value?.archiveSha256 === expectedSha, "Strict reopen returned a different archive digest.");
  assert(outcome.value?.sourceRecheckedUnchanged === true, "Strict reopen did not recheck its source identity.");
  return outcome.value;
}

async function saveAndReopen(page, result, snapshot, destination) {
  const saved = await nativeInvoke(page, "save_internal_project_archive_v6_copy", {
    request: {
      surface: STANDARD_MULTIMOD_SURFACE,
      experimentalLabsEnabled: false,
      sourceArchivePath: result.archivePath,
      expectedSourceArchiveSha256: result.archiveSha256,
      destinationArchivePath: destination,
      project: snapshot.project,
    },
  });
  assert(saved?.status === "ok", `Archive V6 Save Copy failed: ${JSON.stringify(saved?.diagnostic ?? {})}`);
  assert(saved.value?.persistence === "persisted_new_copy", "Archive V6 Save Copy did not use no-replace persistence.");
  const reopened = await inspectStrict(page, destination, saved.value.snapshot.archiveSha256);
  assert(reopened.project?.multimod_results?.some((entry) => entry.result_id === result.attachment.result_id), "Saved copy omitted the exact completed MultiMod result.");
  return { path: destination, sha256: saved.value.snapshot.archiveSha256, bytes: saved.value.snapshot.archiveBytes, strict_reopen_validated: true };
}

async function exportRawSidecar(page, result, directory, stem) {
  const exportable = result.attachment.sidecars.find((entry) => /-(?:posteriors|memberships|assignments|hard-assignments|ledger|target-vectors|draw-rows|records|counts|usable-indices)\.arrow$/u.test(entry.entry_name));
  assert(exportable, "Completed result has no supported raw Arrow evidence export.");
  fs.mkdirSync(directory, { recursive: true });
  const destination = path.join(directory, `${stem}.arrow`);
  const receipt = await nativeInvoke(page, "publish_internal_labs_multimod_raw_sidecar_v1", {
    request: {
      schemaVersion: 1,
      surface: STANDARD_MULTIMOD_SURFACE,
      experimentalLabsEnabled: false,
      archivePath: result.archivePath,
      expectedArchiveSha256: result.archiveSha256,
      projectId: result.projectId,
      resultId: result.attachment.result_id,
      entryName: exportable.entry_name,
      expectedIdentitySha256: exportable.identity_sha256,
      expectedPayloadSha256: exportable.sha256,
      destinationPath: destination,
    },
  });
  assert(receipt?.strictReopenValidated && receipt.noReplacePublication, "Raw sidecar export skipped strict no-replace publication.");
  assert(hashFile(destination) === receipt.sha256 && receipt.sha256 === exportable.sha256, "Raw sidecar export digest differs from its descriptor.");
  return receipt;
}

async function integrityVariants(page, fixtureRoot, result) {
  const variants = await nativeInvoke(page, "prepare_multimod_packaged_integrity_variants_v1", {
    request: {
      surface: STANDARD_MULTIMOD_SURFACE,
      experimentalLabsEnabled: false,
      fixtureRoot,
      archivePath: result.archivePath,
      expectedArchiveSha256: result.archiveSha256,
      resultId: result.attachment.result_id,
    },
  });
  assert(variants?.productionStrictReopenRejectedMissing && variants.productionStrictReopenRejectedTamper, "Fixture creator did not observe strict sidecar integrity rejection.");
  for (const [label, archivePath] of [["missing", variants.missingSidecarArchivePath], ["tampered", variants.tamperedSidecarArchivePath]]) {
    const blocked = await nativeInvoke(page, "inspect_internal_project_archive_v6_zip", {
      request: { surface: STANDARD_MULTIMOD_SURFACE, experimentalLabsEnabled: false, archivePath },
    });
    assert(blocked?.status === "blocked", `Production strict reopen accepted the ${label}-sidecar archive.`);
  }
  return variants;
}

async function openStandardMultiModWorkspace(page, family) {
  const activationTimeout = Math.min(
    120_000,
    remainingScientificMilliseconds("Standard MultiMod UI activation"),
  );
  await page.evaluate(({ archivePath }) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", {
      detail: { path: archivePath },
    }));
  }, { archivePath: family.archive_path });
  const action = page.locator('[data-testid="native-multimod-workspace-open"]');
  await action.waitFor({ state: "visible", timeout: activationTimeout });
  await action.click({ timeout: activationTimeout });
  const standardBadge = page.locator(".nd-multimod-labs-badge.standard", {
    hasText: "Standard · Release-qualified",
  });
  await standardBadge.waitFor({ state: "visible", timeout: activationTimeout });
  return {
    archive_path: family.archive_path,
    archive_sha256: family.archive_sha256,
    result_id: family.result_id,
    production_project_open_event: "quickpls:open-project-path",
    production_toolbar_action: "[data-testid=native-multimod-workspace-open]",
    standard_badge_visible: true,
  };
}

const args = parseArguments(process.argv.slice(2));
const driverStartedEpochMs = Date.now();
const scientificDeadlineEpochMs = Number(args["scientific-deadline-epoch-ms"]);
assert(
  Number.isSafeInteger(scientificDeadlineEpochMs)
    && scientificDeadlineEpochMs > driverStartedEpochMs,
  "The driver-wide scientific deadline must be a future JavaScript-safe epoch millisecond.",
);
assert(/^[a-f0-9]{40}$/u.test(args["candidate-commit"]), "Candidate commit binding is invalid.");
assert(args["candidate-version"] === "2.56.0", "Candidate version binding is invalid.");
for (const digest of ["candidate-sha256", "plan-sha256", "binding-sha256", "authority-document-sha256", "authority-binding-sha256", "prepackage-manifest-set-sha256", "package-receipt-sha256"]) {
  assert(/^[a-f0-9]{64}$/u.test(args[digest]), `${digest} binding is invalid.`);
}
assert(/^\d+$/u.test(args.seed), "Seed binding is invalid.");
const numericSeed = Number(args.seed);
assert(Number.isSafeInteger(numericSeed) && numericSeed >= 0, "Packaged workflow seed must be an unsigned JavaScript-safe integer.");
assertWithinScientificDeadline("Packaged workflow initialization");
const output = path.resolve(args.output);
const outputDirectory = path.dirname(output);
const workRoot = path.join(outputDirectory, `${args.kind}-production-workflows`);
assert(!fs.existsSync(workRoot), `Packaged workflow root already exists: ${workRoot}`);
fs.mkdirSync(workRoot, { recursive: false });
const screenshot = path.join(workRoot, `${args.kind}-completed-workflows.png`);
const {
  browser,
  page: connectedPage,
} = await connectToSingleQuickPlsPage({
  chromium,
  endpoint: args.endpoint,
  expectedOrigin: PACKAGED_TAURI_ORIGIN,
  attempts: 480,
  intervalMilliseconds: 250,
});
const consoleErrors = [];
const functionalNetwork = [];
let page = connectedPage;
try {
  page.on("console", (message) => { if (message.type() === "error") consoleErrors.push(message.text()); });
  page.on("pageerror", (error) => consoleErrors.push(String(error)));
  page.on("request", (request) => {
    const url = request.url();
    const parsed = new URL(url);
    if (!["app:", "tauri:", "data:", "blob:"].includes(parsed.protocol)
      && !["127.0.0.1", "localhost", "ipc.localhost"].includes(parsed.hostname)) functionalNetwork.push(url);
  });
  await page.waitForLoadState("domcontentloaded");
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 45_000 });
  assert((await page.title()).includes("QuickPLS"), "Packaged page title does not identify QuickPLS.");
  await page.evaluate(() => localStorage.setItem("quickpls:native-ui-preferences:v1", JSON.stringify({ experimentalLabsEnabled: false })));
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 45_000 });

  const registry = await nativeInvoke(page, "capability_registry_v2");
  assert(registry?.registry_id === "quickpls.capability_registry.v2" && /^[a-f0-9]{64}$/u.test(registry?.source_sha256 ?? ""), "Embedded capability registry is invalid.");
  const authority = await nativeInvoke(page, "multimod_candidate_authority_status_v1");
  assert(authority?.schemaVersion === 1 && authority.state === "release_qualified_candidate", "Packaged executable has no candidate authority.");
  assert(authority.standardSurfaceAuthorized === true, "Embedded authority did not authorize the Standard MultiMod surface.");
  assert(authority.embeddedDocumentSha256 === args["authority-document-sha256"], "Embedded authority document differs from package receipt.");
  assert(authority.authorityBindingSha256 === args["authority-binding-sha256"], "Embedded authority binding differs from package receipt.");
  assert(authority.candidateCommitSha === args["candidate-commit"] && authority.candidateVersion === args["candidate-version"], "Embedded candidate identity differs from package receipt.");
  assert(authority.qualificationPlanSha256 === args["plan-sha256"] && authority.gateBindingSha256 === args["binding-sha256"], "Embedded plan/binding differs from package receipt.");
  assert(authority.prepackageManifestSetSha256 === args["prepackage-manifest-set-sha256"], "Embedded manifest set differs from package receipt.");
  assert(Array.isArray(authority.exactProfileCells) && authority.exactProfileCells.length > 0, "Embedded authority has no exact cells.");
  assert(authority.exactProfileCells.every((cell, index, cells) => /^[a-z0-9][a-z0-9._-]*::[a-z0-9][a-z0-9._-]*$/u.test(cell) && (index === 0 || cells[index - 1] < cell)), "Embedded authority cells are not exact, sorted, and unique.");

  const bridgeIdentity = await page.evaluate(() => {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, "__QPLS_MULTIMOD_PACKAGED_QUALIFICATION_V1__");
    const bridge = descriptor?.value;
    return { schemaVersion: bridge?.schemaVersion, bridgeId: bridge?.bridgeId, frozen: Object.isFrozen(bridge), configurable: descriptor?.configurable, enumerable: descriptor?.enumerable, writable: descriptor?.writable };
  });
  assert(bridgeIdentity.schemaVersion === 1 && bridgeIdentity.bridgeId === "qpls.v256.multimod.packaged-qualification-bridge.v1", "Build-only qualification bridge identity is missing.");
  assert(bridgeIdentity.frozen && bridgeIdentity.configurable === false && bridgeIdentity.enumerable === false && bridgeIdentity.writable === false, "Build-only qualification bridge is mutable.");

  const invalidPreflight = await expectInvokeRejected(page, "preflight_internal_labs_multimod_v1", { request: {} });
  assert(invalidPreflight.rejected, "Malformed MultiMod preflight did not fail closed.");
  const fixtureRoot = path.join(workRoot, "fixtures");
  const prepared = await nativeInvoke(page, "prepare_multimod_packaged_qualification_fixtures_v1", {
    request: { surface: STANDARD_MULTIMOD_SURFACE, experimentalLabsEnabled: false, outputDirectory: fixtureRoot, seed: numericSeed },
  });
  assert(prepared?.fixtureId === "qpls.v256.multimod.packaged-production-fixtures.v1", "Packaged production fixture identity is invalid.");
  assert(prepared.families?.length === 4 && new Set(prepared.families.map((fixture) => fixture.familyId)).size === 4, "Packaged fixture set does not contain four exact result families.");

  const families = [];
  let cancellationRecovery;
  let tamperedAuthorityRejection;
  for (const fixture of prepared.families) {
    const expected = FAMILY_EXPECTATIONS[fixture.familyId];
    assert(expected, `Unexpected packaged fixture ${fixture.familyId}.`);
    const startedAt = Date.now();
    const executed = await runFamily(page, fixture, authority);
    if (executed.cancellation) cancellationRecovery = executed.cancellation;
    const strict = await inspectStrict(page, executed.result.archivePath, executed.result.archiveSha256);
    assert(strict.project?.multimod_results?.some((entry) => entry.result_id === executed.result.attachment.result_id), `${fixture.familyId} strict reopen omitted its result.`);
    const slug = stableSlug(fixture.familyId);
    const saveCopy = await saveAndReopen(page, executed.result, strict, path.join(workRoot, `${slug}-saved-copy.qpls`));
    const exports = await exportCanonicalMatrix(page, executed.result.canonicalDocument, path.join(workRoot, `${slug}-exports`), slug);
    const rawSidecar = await exportRawSidecar(page, executed.result, path.join(workRoot, `${slug}-raw`), `${slug}-raw-evidence`);
    const integrity = await integrityVariants(page, fixtureRoot, executed.result);
    if (!tamperedAuthorityRejection) tamperedAuthorityRejection = await verifyTamperedCandidateReceiptFails(page, executed.result.canonicalDocument, path.join(workRoot, "tampered-authority-export"), "tampered-authority");
    families.push({
      family_id: fixture.familyId,
      target: expected.target,
      result_kind: expected.resultKind,
      valid_run_completed: true,
      result_identity_verified: true,
      result_id: executed.result.attachment.result_id,
      archive_path: executed.result.archivePath,
      archive_sha256: executed.result.archiveSha256,
      archive_reopened: true,
      save_copy: saveCopy,
      semantic_export_readback: true,
      exports,
      raw_sidecar_export: rawSidecar,
      sidecar_integrity_verified: true,
      integrity_variants: integrity,
      authority_receipt_verified: true,
      elapsed_milliseconds: Date.now() - startedAt,
    });
  }
  assert(cancellationRecovery, "The packaged MGA workflow did not exercise cancellation and deterministic resume.");
  const chartFormats = new Set(families.flatMap((family) => family.exports.map((entry) => entry.format)));
  assert(chartFormats.has("svg") && chartFormats.has("png"), "The four-family canonical matrix exposed no chart for SVG/PNG semantic publication and readback.");

  const multiModUiRoute = await openStandardMultiModWorkspace(page, families[0]);

  assertWithinScientificDeadline("Packaged accessibility checks");
  const accessibility = await page.evaluate(() => {
    const visible = (element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
    };
    const controls = [...document.querySelectorAll("button, a[href], input, select, textarea")].filter(visible);
    const unnamed = controls.filter((element) => !element.getAttribute("aria-label")?.trim()
      && !element.textContent?.trim() && !element.getAttribute("title")?.trim()
      && !element.getAttribute("aria-labelledby"));
    return {
      main_count: document.querySelectorAll("main, [role='main']").length,
      heading_count: document.querySelectorAll("h1, h2, h3, [role='heading']").length,
      control_count: controls.length,
      unnamed_control_count: unnamed.length,
      live_region_count: document.querySelectorAll("[aria-live], [role='status'], [role='alert']").length,
    };
  });
  assert(accessibility.heading_count > 0 && accessibility.control_count > 0 && accessibility.unnamed_control_count === 0, "Packaged workflow surface failed the accessible-name/heading checks.");
  await page.keyboard.press("Tab");
  const keyboardFocusReached = await page.evaluate(() => document.activeElement && document.activeElement !== document.body && document.activeElement !== document.documentElement);
  assert(keyboardFocusReached, "Keyboard Tab did not reach a visible control after workflow completion.");
  const multiModSurfacePresentation = await page.evaluate(() => {
    const badgeTexts = [...document.querySelectorAll(".nd-multimod-labs-badge")]
      .filter((element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
      })
      .map((element) => element.textContent?.trim() ?? "");
    const labsPattern = /(?:Experimental\s+Labs|Lab\s+Only|Labs\s+Only|Labs\s*·\s*Unqualified)/iu;
    return {
      badge_texts: badgeTexts,
      standard_badge_count: badgeTexts.filter((text) => text === "Standard · Release-qualified").length,
      lab_badge_count: badgeTexts.filter((text) => labsPattern.test(text)).length,
    };
  });
  assert(multiModSurfacePresentation.standard_badge_count >= 1, "The qualified Standard MultiMod surface was not visibly rendered.");
  assert(multiModSurfacePresentation.lab_badge_count === 0, "A visible MultiMod Lab-only badge remained on the qualified Standard surface.");
  await page.screenshot({ path: screenshot, fullPage: true });

  const resources = await page.evaluate(() => performance.getEntriesByType("resource").map((entry) => entry.name));
  const remoteResources = resources.filter((url) => {
    try {
      const parsed = new URL(url);
      return !["app:", "tauri:", "data:", "blob:"].includes(parsed.protocol)
        && !["127.0.0.1", "localhost", "ipc.localhost"].includes(parsed.hostname);
    } catch { return false; }
  });
  assert(functionalNetwork.length === 0 && remoteResources.length === 0, "Packaged candidate attempted non-loopback network access during real workflows.");
  assert(consoleErrors.length === 0, `Packaged candidate emitted renderer errors: ${consoleErrors.join(" | ")}`);
  const screenshotBytes = fs.readFileSync(screenshot);
  const completedEpochMs = Date.now();
  assert(completedEpochMs <= scientificDeadlineEpochMs, "Packaged workflow completed after its driver-wide scientific deadline.");
  const report = {
    schema_version: 1,
    report_id: "qpls.v256.multimod.packaged-offline-production-smoke.v1",
    passed: true,
    qualification_coverage_complete: true,
    coverage_scope: "exact_packaged_candidate_four_family_production_workflows",
    package_kind: args.kind,
    candidate_commit_sha: args["candidate-commit"],
    candidate_version: args["candidate-version"],
    plan_sha256: args["plan-sha256"],
    binding_sha256: args["binding-sha256"],
    authority_document_sha256: args["authority-document-sha256"],
    authority_binding_sha256: args["authority-binding-sha256"],
    manifest_set_sha256: args["prepackage-manifest-set-sha256"],
    package_receipt_sha256: args["package-receipt-sha256"],
    seed: numericSeed,
    candidate: { path: path.resolve(args["candidate-path"]), sha256: args["candidate-sha256"], pid: Number(args["candidate-pid"]) },
    harness: {
      bridge: bridgeIdentity,
      compile_time_feature_required: true,
      embedded_candidate_authority_required: true,
      runtime_or_request_authority_injection: false,
      unmerged_review_candidate: true,
      later_harness_disabled_rebuild_covered: false,
    },
    native_registry_digest: registry.source_sha256,
    embedded_candidate_authority: authority,
    standard_surface_verified: true,
    labs_opt_in_not_required: true,
    lab_badge_absent: multiModSurfacePresentation.lab_badge_count === 0,
    multimod_surface_presentation: multiModSurfacePresentation,
    multimod_production_ui_route: multiModUiRoute,
    invalid_preflight_failed_closed: true,
    candidate_receipt_tamper_failed_closed: tamperedAuthorityRejection.rejected,
    families,
    cancellation_recovery: cancellationRecovery,
    cancellation_recovery_verified: true,
    accessibility,
    keyboard_focus_reached: true,
    offline: { passed: true, functional_network_requests: functionalNetwork, remote_resource_urls: remoteResources },
    console_errors: consoleErrors,
    screenshot: { path: screenshot, sha256: sha256(screenshotBytes), size: screenshotBytes.length },
    timing: {
      contract_id: "qpls.v256.multimod.packaged-driver-scientific-deadline.v1",
      driver_started_epoch_ms: driverStartedEpochMs,
      scientific_deadline_epoch_ms: scientificDeadlineEpochMs,
      completed_epoch_ms: completedEpochMs,
      elapsed_milliseconds: completedEpochMs - driverStartedEpochMs,
      remaining_milliseconds_at_publication: scientificDeadlineEpochMs - completedEpochMs,
      poll_deadlines_clamped_to_family_and_driver: true,
      late_exit_rejection_enabled: true,
    },
    generated_at_utc: new Date().toISOString(),
  };
  writeJsonAtomic(output, report);
  assert(Date.now() <= scientificDeadlineEpochMs, "Packaged report publication crossed the driver-wide scientific deadline.");
  process.stdout.write(`${JSON.stringify(report)}\n`);
} finally {
  await browser.close();
}
