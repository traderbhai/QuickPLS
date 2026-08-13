import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const resultsRoot = path.join(root, "validation", "results");
const reportPath = path.join(resultsRoot, "diagnostic_bundle_packaged_acceptance.raw.json");
const screenshotDir = path.join(resultsRoot, "screens", "diagnostic-bundle-packaged-acceptance");
const saveHelperPath = path.join(root, "validation", "windows_native_save_diagnostic_bundle.py");
const endpoint = process.env.QUICKPLS_CDP_ENDPOINT?.trim() || "http://127.0.0.1:9222";
const pythonExecutable = process.env.QUICKPLS_PYTHON?.trim() || "python";
const requestedZipPath = process.env.QUICKPLS_DIAGNOSTIC_ZIP_PATH?.trim() || "";
const testedDesktopExecutable = process.env.QUICKPLS_DESKTOP_EXE_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_DESKTOP_EXE_PATH.trim())
  : path.join(root, "target", "release", "quickpls-desktop.exe");
const packagedOrigin = "http://tauri.localhost";
const ipcOrigin = "http://ipc.localhost";
const allowedPageOrigins = [ipcOrigin, packagedOrigin];
const expectedIncludedCategories = [
  "QuickPLS build and release identity",
  "Operating-system family and architecture",
  "Bounded session diagnostic event codes",
  "Manifest hashes, sizes, limits, and redaction counts",
];
const expectedExcludedCategories = [
  "Dataset rows, values, and variable names",
  "Project contents, model labels, and project titles",
  "Results, reports, and exports",
  "Credentials, environment values, and command lines",
  "Arbitrary files, registry data, and memory dumps",
];
const expectedArchiveEntries = ["metadata/system.json", "logs/events.jsonl", "manifest.json"];
const expectedPayloadEntries = expectedArchiveEntries.slice(0, 2);

const evidence = {
  schema_version: "quickpls.diagnostic_bundle_packaged_acceptance.raw.v1",
  kind: "quickpls3_packaged_diagnostic_bundle_v1_raw_acceptance",
  passed: false,
  generated_at_utc: new Date().toISOString(),
  target: "windows_10_11_x64_packaged_tauri",
  runtime: {
    node: process.version,
    platform: process.platform,
    architecture: process.arch,
    playwright: "chromium-connect-over-cdp",
  },
  endpoint,
  generator: "validation/diagnostic_bundle_packaged_acceptance.mjs",
  tested_product: null,
  checks: {},
  artifacts: {},
  browser_requests: [],
  console_errors: [],
  failures: [],
};

function assert(condition, message, context = undefined) {
  if (!condition) {
    throw new Error(context === undefined ? message : `${message}: ${JSON.stringify(context)}`);
  }
}

function relativePath(value) {
  return path.relative(root, value).replaceAll("\\", "/");
}

async function artifactDigest(value) {
  const bytes = await fs.readFile(value);
  return {
    path: relativePath(value),
    size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

async function validateNewZipPath(value) {
  assert(path.isAbsolute(value), "QUICKPLS_DIAGNOSTIC_ZIP_PATH must be absolute");
  const resolved = path.resolve(value);
  assert(path.extname(resolved).toLocaleLowerCase() === ".zip", "Diagnostic target must use .zip");
  const realRoot = await fs.realpath(resultsRoot);
  const realParent = await fs.realpath(path.dirname(resolved));
  const relative = path.relative(realRoot, realParent);
  assert(relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative), "Diagnostic target must resolve inside validation/results");
  try {
    await fs.access(resolved);
    throw new Error(`Diagnostic target must not already exist: ${resolved}`);
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("Diagnostic target")) throw error;
    if (error?.code !== "ENOENT") throw error;
  }
  return resolved;
}

function helperFailure(message, context = {}) {
  return { event: "complete", passed: false, phase: "helper_transport", error: { type: "HelperTransportError", message }, ...context };
}

function startNativeSaveHelper(targetPath, windowTitle) {
  const child = spawn(pythonExecutable, [
    saveHelperPath,
    "--target", targetPath,
    "--results-root", resultsRoot,
    "--window-title", windowTitle,
    "--timeout-seconds", "45",
  ], { cwd: root, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  const events = [];
  const protocolErrors = [];
  let stdoutBuffer = "";
  let stderr = "";
  let finalEvent = null;
  let exited = false;
  let readySettled = false;
  let completeSettled = false;
  let resolveReady;
  let resolveComplete;
  const ready = new Promise((resolve) => { resolveReady = resolve; });
  const completed = new Promise((resolve) => { resolveComplete = resolve; });
  const settleReady = (value) => { if (!readySettled) { readySettled = true; resolveReady(value); } };
  const settleComplete = (value) => { if (!completeSettled) { completeSettled = true; resolveComplete(value); } };
  const consumeLine = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      events.push(event);
      if (event?.event === "ready") settleReady(event);
      if (event?.event === "complete") {
        finalEvent = event;
        if (!event.passed) settleReady(event);
      }
    } catch (error) {
      protocolErrors.push({ line, error: error instanceof Error ? error.message : String(error) });
    }
  };
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdoutBuffer += chunk;
    const lines = stdoutBuffer.split(/\r?\n/);
    stdoutBuffer = lines.pop() ?? "";
    lines.forEach(consumeLine);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.on("error", (error) => {
    const failure = helperFailure(error.message, { code: error.code ?? null });
    settleReady(failure);
    settleComplete({ ...failure, transport: { stderr, events, protocolErrors } });
  });
  child.on("close", (code, signal) => {
    exited = true;
    if (stdoutBuffer.trim()) consumeLine(stdoutBuffer);
    let result = finalEvent ?? helperFailure("The native diagnostic Save helper exited without a completion event.");
    if (result.passed && (code !== 0 || protocolErrors.length > 0)) {
      result = helperFailure("The native diagnostic Save helper reported success with invalid transport state.", { reportedCompletion: result });
    }
    const transported = { ...result, transport: { exitCode: code, signal, stderr: stderr.trim(), events, protocolErrors } };
    settleReady(result.passed ? helperFailure("The helper exited before readiness.") : result);
    settleComplete(transported);
  });
  const timeout = setTimeout(() => {
    if (exited) return;
    child.kill();
    const failure = helperFailure("The native diagnostic Save helper exceeded 70 seconds.");
    settleReady(failure);
    settleComplete({ ...failure, transport: { stderr: stderr.trim(), events, protocolErrors } });
  }, 70_000);
  completed.finally(() => clearTimeout(timeout));
  return { ready, completed, stop: () => { if (!exited) child.kill(); } };
}

async function inspectCdpPage(candidate, index) {
  const fallbackUrl = candidate.url();
  try {
    const state = await candidate.evaluate(() => {
      const shell = document.querySelector(".nd-app[data-native-desktop-shell='true']");
      const style = shell ? getComputedStyle(shell) : null;
      return {
        title: document.title,
        shell_visible: Boolean(shell && style?.display !== "none" && style?.visibility !== "hidden" && shell.getClientRects().length > 0),
        tauri_runtime: Boolean(window.__TAURI_INTERNALS__),
      };
    });
    const url = candidate.url();
    return { index, url, origin: new URL(url).origin, ...state };
  } catch {
    let origin = null;
    try { origin = new URL(fallbackUrl).origin; } catch { /* invalid URL stays null */ }
    return { index, url: fallbackUrl, origin, title: "", shell_visible: false, tauri_runtime: false };
  }
}

async function enumeratePages(browser) {
  const pages = browser.contexts().flatMap((context) => context.pages());
  const states = await Promise.all(pages.map((candidate, index) => inspectCdpPage(candidate, index)));
  return pages.map((candidate, index) => ({ candidate, state: states[index] }));
}

async function invokeNative(page, command, args) {
  return page.evaluate(async ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

async function captureExactIpcJsonForAction(
  page,
  command,
  expectedRequestPayload,
  action,
  settled,
  timeout = 10_000,
) {
  const expectedUrl = `http://ipc.localhost/${encodeURIComponent(command)}`;
  const matchingRequests = [];
  const matchingResponses = [];
  const matchesRequest = (request) => request.url() === expectedUrl
    && request.method() === "POST"
    && request.resourceType() === "fetch";
  const matchesResponse = (response) => matchesRequest(response.request());
  const onRequest = (request) => { if (matchesRequest(request)) matchingRequests.push(request); };
  const onResponse = (response) => { if (matchesResponse(response)) matchingResponses.push(response); };
  page.on("request", onRequest);
  page.on("response", onResponse);
  const responsePromise = page.waitForResponse(matchesResponse, { timeout });
  try {
    const [response] = await Promise.all([responsePromise, Promise.resolve().then(action)]);
    await settled();
    await page.waitForTimeout(250);
    assert(matchingRequests.length === 1, `Expected exactly one ${command} IPC request`, { count: matchingRequests.length });
    assert(matchingResponses.length === 1, `Expected exactly one ${command} IPC response`, { count: matchingResponses.length });
    assert(matchingResponses[0] === response && response.request() === matchingRequests[0], `${command} IPC request/response identity was ambiguous`);
    let requestPayload;
    try {
      requestPayload = response.request().postDataJSON();
    } catch (error) {
      throw new Error(`${command} IPC request body was not valid JSON: ${error instanceof Error ? error.message : String(error)}`);
    }
    assert(
      canonicalJson(requestPayload) === canonicalJson(expectedRequestPayload),
      `${command} IPC request arguments did not match the UI action`,
      { expected: expectedRequestPayload, actual: requestPayload },
    );
    assert(response.ok(), `${command} IPC response was not HTTP-successful`, { status: response.status() });
    const headers = response.headers();
    assert(headers["tauri-response"] === "ok", `${command} IPC response did not carry Tauri-Response: ok`, headers);
    const contentType = (headers["content-type"] ?? "").split(/[;,]/, 1)[0].trim().toLowerCase();
    assert(contentType === "application/json", `${command} IPC response was not application/json`, { contentType });
    try {
      return await response.json();
    } catch (error) {
      throw new Error(`${command} IPC response body was not valid JSON: ${error instanceof Error ? error.message : String(error)}`);
    }
  } finally {
    page.off("request", onRequest);
    page.off("response", onResponse);
  }
}

function requireDiagnosticPreviewPayload(preview) {
  assert(preview && typeof preview === "object" && !Array.isArray(preview), "The live UI preview IPC response was not an object");
  assert(/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(preview.previewId ?? ""), "The live UI preview IPC response did not contain a UUID previewId", preview);
  assert(preview.stagedContents && typeof preview.stagedContents === "object", "The live UI preview IPC response omitted stagedContents", preview);
  assert(Array.isArray(preview.stagedContents.events), "The live UI preview IPC response omitted staged event rows", preview);
  assert(preview.stagedContents.system && preview.stagedContents.manifest, "The live UI preview IPC response omitted system or manifest contents", preview);
  return preview;
}

async function openPreferences(page) {
  const applicationMenu = page.getByRole("menubar", { name: "Application menu" });
  const toolsTrigger = applicationMenu.getByRole("menuitem", { name: "Tools", exact: true });
  await toolsTrigger.click();
  await page.getByRole("menu", { name: "Tools", exact: true })
    .getByRole("menuitem", { name: /^Preferences/ })
    .click();
  const dialog = page.getByRole("dialog", { name: "Preferences" });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.locator('[data-live-preferences-dialog="true"] [data-diagnostic-bundle-panel="live"]').waitFor({ state: "visible", timeout: 10_000 });
  return dialog;
}

async function previewFromUi(dialog, page) {
  const preview = await captureExactIpcJsonForAction(
    page,
    "preview_diagnostic_bundle",
    { replacesPreviewId: null },
    () => dialog.getByRole("button", { name: /^Preview bundle/ }).click(),
    () => dialog.getByText("Preview ready. Review the included and excluded categories before saving.", { exact: true })
      .waitFor({ state: "visible", timeout: 10_000 }),
  );
  return requireDiagnosticPreviewPayload(preview);
}

function cancellationEventCount(preview) {
  return preview.stagedContents.events.filter((row) => row.code === "diagnostic.preview.cancelled").length;
}

async function expectedSaveRejection(page, caseId, pathValue, expectedCode) {
  const preview = await invokeNative(page, "preview_diagnostic_bundle", { replacesPreviewId: null });
  let observedError = "";
  try {
    await invokeNative(page, "save_diagnostic_bundle", { path: pathValue, previewId: preview.previewId });
  } catch (error) {
    observedError = String(error);
  }
  assert(observedError.includes(expectedCode), `Destination rejection ${caseId} did not return ${expectedCode}`, observedError);
  let consumedError = "";
  try {
    await invokeNative(page, "cancel_diagnostic_bundle_preview", { previewId: preview.previewId });
  } catch (error) {
    consumedError = String(error);
  }
  assert(consumedError.includes("DIAGNOSTIC_PREVIEW_REQUIRED"), `Destination rejection ${caseId} did not consume its one-time preview`, consumedError);
  return { case_id: caseId, path: pathValue, expected_code: expectedCode, observed_error: observedError, preview_consumed: true };
}

await fs.mkdir(screenshotDir, { recursive: true });
await fs.mkdir(resultsRoot, { recursive: true });
let browser = null;
let nativeSaveHelper = null;
let temporaryExistingPath = null;
try {
  assert(process.platform === "win32", "Diagnostic packaged acceptance requires Windows");
  const targetZip = await validateNewZipPath(requestedZipPath);
  evidence.tested_product = { quickpls_desktop_exe: await artifactDigest(testedDesktopExecutable) };

  browser = await chromium.connectOverCDP(endpoint);
  let pageEntries = [];
  for (let attempt = 0; attempt < 60; attempt += 1) {
    pageEntries = await enumeratePages(browser);
    if (pageEntries.some(({ state }) => state.shell_visible && state.tauri_runtime)) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const qualifying = pageEntries.filter(({ state }) => state.shell_visible && state.tauri_runtime);
  assert(qualifying.length === 1, "Expected exactly one visible QuickPLS Tauri CDP page", pageEntries.map(({ state }) => state));
  const page = qualifying[0].candidate;
  page.on("pageerror", (error) => evidence.console_errors.push({ type: "pageerror", message: error.message }));
  page.on("console", (message) => { if (message.type() === "error") evidence.console_errors.push({ type: "console", message: message.text() }); });
  page.on("request", (request) => {
    const url = request.url();
    let origin = null;
    try { origin = new URL(url).origin; } catch { /* non-URL resource */ }
    evidence.browser_requests.push({ url, origin, method: request.method(), resource_type: request.resourceType() });
  });
  const beforeReload = qualifying[0].state;
  await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  const afterReload = await inspectCdpPage(page, beforeReload.index);
  evidence.checks.runtime_preflight = {
    passed: afterReload.origin === packagedOrigin && afterReload.shell_visible && afterReload.tauri_runtime,
    expected_origin: packagedOrigin,
    qualifying_page_count: qualifying.length,
    pre_reload: beforeReload,
    reload_count: 1,
    post_reload: afterReload,
    same_origin: beforeReload.origin === afterReload.origin,
  };
  assert(evidence.checks.runtime_preflight.passed && evidence.checks.runtime_preflight.same_origin, "Packaged Tauri runtime preflight failed", evidence.checks.runtime_preflight);
  const abandonedPreviews = [];
  for (let index = 0; index < 7; index += 1) {
    abandonedPreviews.push(await invokeNative(page, "preview_diagnostic_bundle", { replacesPreviewId: null }));
  }
  let dialog = await openPreferences(page);
  const initialSaveDisabled = await dialog.getByRole("button", { name: /^Save new ZIP/ }).isDisabled();
  const initialCancelDisabled = await dialog.getByRole("button", { name: /^Cancel preview/ }).isDisabled();
  const recoveredPreview = await previewFromUi(dialog, page);
  const evicted = [];
  for (const preview of abandonedPreviews.slice(0, 4)) {
    let error = "";
    try { await invokeNative(page, "cancel_diagnostic_bundle_preview", { previewId: preview.previewId }); } catch (reason) { error = String(reason); }
    assert(error.includes("DIAGNOSTIC_PREVIEW_REQUIRED"), "The oldest abandoned preview remained consumable after bounded eviction", { id: preview.previewId, error });
    evicted.push({ preview_id: preview.previewId, error });
  }
  const survivorCancellations = [];
  for (const preview of abandonedPreviews.slice(4)) {
    await invokeNative(page, "cancel_diagnostic_bundle_preview", { previewId: preview.previewId });
    survivorCancellations.push(preview.previewId);
  }
  evidence.checks.abandoned_preview_recovery = {
    passed: true,
    abandoned_preview_count: abandonedPreviews.length,
    backend_capacity: 4,
    evicted_oldest_count: evicted.length,
    evicted_oldest: evicted,
    surviving_abandoned_cancelled_count: survivorCancellations.length,
    surviving_abandoned_cancelled_ids: survivorCancellations,
    live_ui_recovered_at_capacity: Boolean(recoveredPreview.previewId),
  };

  const panel = dialog.locator('[data-diagnostic-bundle-panel="live"]');
  const livePreviewScreenshot = path.join(screenshotDir, "01-live-preferences-diagnostic-preview.png");
  await dialog.screenshot({ path: livePreviewScreenshot });
  const previewContents = recoveredPreview.stagedContents;
  const eventRegion = panel.getByRole("region", { name: "Redacted diagnostic event rows" });
  const descriptorRegion = panel.getByRole("region", { name: "Diagnostic manifest payload descriptors" });
  const descriptorNames = await descriptorRegion.locator("tbody tr td:first-child").allTextContents();
  const includedText = (await panel.getByText("Included in this preview", { exact: true }).locator("..").textContent()) ?? "";
  const excludedText = (await panel.getByText("Always excluded", { exact: true }).locator("..").textContent()) ?? "";
  const systemLabels = await panel.locator('[aria-label="Redacted staged diagnostic contents"] dl dt').allTextContents();
  const systemValues = await panel.locator('[aria-label="Redacted staged diagnostic contents"] dl dd').allTextContents();
  const expectedSystemLabels = ["Metadata schema", "QuickPLS version", "Release channel", "Source revision", "Operating system", "Architecture", "Desktop runtime", "Locale", "WebView2 version", "User data included", "Network accessed"];
  assert(recoveredPreview.localOnly === true && recoveredPreview.networkActivity === "none" && recoveredPreview.entryCount === 3, "Live preview locality or entry contract drifted", recoveredPreview);
  assert(JSON.stringify(recoveredPreview.includedCategories) === JSON.stringify(expectedIncludedCategories), "Included diagnostic categories drifted", recoveredPreview.includedCategories);
  assert(JSON.stringify(recoveredPreview.excludedCategories) === JSON.stringify(expectedExcludedCategories), "Excluded diagnostic categories drifted", recoveredPreview.excludedCategories);
  assert(JSON.stringify(descriptorNames) === JSON.stringify(expectedPayloadEntries), "Live manifest descriptor rows drifted", descriptorNames);
  assert(JSON.stringify(systemLabels) === JSON.stringify(expectedSystemLabels), "Live system metadata labels drifted", systemLabels);
  assert(systemValues.at(-2) === "no" && systemValues.at(-1) === "no", "Live system privacy flags were not explicit", systemValues);
  assert(await eventRegion.getAttribute("tabindex") === "0" && await descriptorRegion.getAttribute("tabindex") === "0", "Diagnostic tables are not keyboard-scrollable regions");
  assert(expectedIncludedCategories.every((value) => includedText.includes(value)) && expectedExcludedCategories.every((value) => excludedText.includes(value)), "Live category text did not expose every exact category");
  evidence.checks.live_settings_preview = {
    passed: true,
    production_entry: "src/App.tsx -> NativeDesktopApp -> NativeUtilityDialog -> DiagnosticBundlePanel",
    dialog_title: "Preferences",
    panel_marker: "live",
    existing_preferences_preserved: await dialog.getByLabel("Interface density").isVisible()
      && await dialog.getByLabel("Table density").isVisible()
      && await dialog.getByLabel("Default precision").isVisible(),
    preview_before_save: initialSaveDisabled && initialCancelDisabled,
    local_only: recoveredPreview.localOnly,
    network_activity: recoveredPreview.networkActivity,
    entry_count: recoveredPreview.entryCount,
    event_count: recoveredPreview.eventCount,
    included_categories: recoveredPreview.includedCategories,
    excluded_categories: recoveredPreview.excludedCategories,
    staged_contents: previewContents,
    exact_system_labels: systemLabels,
    exact_descriptor_names: descriptorNames,
    accessible_regions: ["Redacted diagnostic event rows", "Diagnostic manifest payload descriptors"],
    live_status_region: await panel.locator('[role="status"][aria-live="polite"][aria-atomic="true"]').count() === 1,
  };
  assert(evidence.checks.live_settings_preview.existing_preferences_preserved && evidence.checks.live_settings_preview.preview_before_save && evidence.checks.live_settings_preview.live_status_region, "Live Preferences integration contract failed", evidence.checks.live_settings_preview);

  await dialog.getByRole("button", { name: /^Cancel preview/ }).click();
  await dialog.getByText("Diagnostic preview cancelled. No file was created.", { exact: true }).waitFor({ state: "visible", timeout: 10_000 });
  const explicitCancellation = { passed: true, preview_id: recoveredPreview.previewId, no_file_created_message: true };

  let navigationPreview = await previewFromUi(dialog, page);
  let priorCancellationCount = cancellationEventCount(navigationPreview);
  const navigationCycles = [];
  for (let cycle = 1; cycle <= 5; cycle += 1) {
    const closingPreviewId = navigationPreview.previewId;
    await dialog.getByRole("button", { name: "Close dialog" }).click();
    await dialog.waitFor({ state: "detached", timeout: 10_000 });
    await new Promise((resolve) => setTimeout(resolve, 250));
    dialog = await openPreferences(page);
    navigationPreview = await previewFromUi(dialog, page);
    const nextCancellationCount = cancellationEventCount(navigationPreview);
    assert(nextCancellationCount > priorCancellationCount, "Closing live Preferences did not record unmount cancellation", { cycle, priorCancellationCount, nextCancellationCount });
    navigationCycles.push({ cycle, closed_preview_id: closingPreviewId, next_preview_id: navigationPreview.previewId, cancellation_event_count_before: priorCancellationCount, cancellation_event_count_after: nextCancellationCount });
    priorCancellationCount = nextCancellationCount;
  }
  evidence.checks.navigation_cancellation = {
    passed: true,
    explicit_preview_cancellation: explicitCancellation,
    unmount_cycles: navigationCycles.length,
    every_unmount_recorded: navigationCycles.every((row) => row.cancellation_event_count_after > row.cancellation_event_count_before),
    recovery_preview_id: navigationPreview.previewId,
  };

  const finalPreview = navigationPreview;
  evidence.checks.live_settings_preview.event_count = finalPreview.eventCount;
  evidence.checks.live_settings_preview.staged_contents = finalPreview.stagedContents;
  nativeSaveHelper = startNativeSaveHelper(targetZip, await page.title());
  const helperReady = await nativeSaveHelper.ready;
  assert(helperReady.passed === true && helperReady.phase === "main_window_binding", "Native diagnostic Save helper did not bind the exact main window", helperReady);
  let helperCompletion;
  const saveResult = await captureExactIpcJsonForAction(
    page,
    "save_diagnostic_bundle",
    { path: targetZip, previewId: finalPreview.previewId },
    () => dialog.getByRole("button", { name: /^Save new ZIP/ }).click(),
    async () => {
      helperCompletion = await nativeSaveHelper.completed;
      assert(helperCompletion.passed === true && helperCompletion.phase === "diagnostic_zip_creation_and_readback", "Native diagnostic Save helper failed", helperCompletion);
      await dialog.getByText(/Diagnostic bundle saved locally \(.+ KiB\)\. QuickPLS did not upload it\./).waitFor({ state: "visible", timeout: 10_000 });
    },
    70_000,
  );
  const savedScreenshot = path.join(screenshotDir, "02-diagnostic-bundle-saved.png");
  await dialog.screenshot({ path: savedScreenshot });
  assert(saveResult?.bytes === helperCompletion.bundle.size && saveResult?.archiveSha256 === helperCompletion.bundle.sha256, "UI save result did not bind the exact archive bytes", { saveResult, bundle: helperCompletion.bundle });
  evidence.checks.native_save_dialog = {
    passed: true,
    target_path: targetZip,
    new_target: true,
    local_drive_rooted: /^[A-Za-z]:[\\/]/.test(targetZip),
    helper_ready: helperReady,
    helper_completion: helperCompletion,
    save_result: saveResult,
    app_feedback: (await panel.getByText(/Diagnostic bundle saved locally/).textContent())?.trim() ?? "",
  };
  assert(evidence.checks.native_save_dialog.local_drive_rooted, "Native diagnostic target was not drive-letter rooted", targetZip);

  const bundle = helperCompletion.bundle;
  const previewArchiveExact = canonicalJson(finalPreview.stagedContents.system) === canonicalJson(bundle.system)
    && canonicalJson(finalPreview.stagedContents.events) === canonicalJson(bundle.events)
    && canonicalJson(finalPreview.stagedContents.manifest) === canonicalJson(bundle.manifest);
  const descriptorsExact = bundle.manifest.entries.every((descriptor) => bundle.entrySizes[descriptor.name] === descriptor.bytes)
    && JSON.stringify(bundle.manifest.entries.map((row) => row.name)) === JSON.stringify(expectedPayloadEntries);
  evidence.checks.archive_integrity = {
    passed: previewArchiveExact && descriptorsExact && bundle.forbiddenPatternMatches.length === 0,
    exact_entry_names: bundle.entryNames,
    exact_entry_count: bundle.entryNames.length,
    stored_compression_only: bundle.entryCompression.every((value) => value === "stored"),
    entry_sizes: bundle.entrySizes,
    uncompressed_bytes: bundle.uncompressedBytes,
    archive_bytes: bundle.size,
    archive_sha256: bundle.sha256,
    manifest_payload_descriptors_exact: descriptorsExact,
    preview_archive_exact: previewArchiveExact,
    redaction_counts: bundle.manifest.redactionCounts,
    redaction_total: bundle.manifest.redactionTotal,
    forbidden_pattern_matches: bundle.forbiddenPatternMatches,
    user_data_included: bundle.system.userDataIncluded,
    network_accessed_declared: bundle.system.networkAccessed,
  };
  assert(evidence.checks.archive_integrity.passed && evidence.checks.archive_integrity.exact_entry_count === 3 && evidence.checks.archive_integrity.stored_compression_only, "Saved diagnostic archive contract failed", evidence.checks.archive_integrity);

  const stamp = `${Date.now()}-${process.pid}`;
  temporaryExistingPath = path.join(resultsRoot, `diagnostic-existing-${stamp}.zip`);
  const existingSentinel = Buffer.from("QuickPLS diagnostic no-overwrite sentinel\n", "utf8");
  await fs.writeFile(temporaryExistingPath, existingSentinel, { flag: "wx" });
  const rejectionCases = [
    await expectedSaveRejection(page, "relative", "relative.zip", "DIAGNOSTIC_PATH_NOT_LOCAL_DRIVE"),
    await expectedSaveRejection(page, "unc", "\\\\server\\share\\bundle.zip", "DIAGNOSTIC_PATH_NAMESPACE_BLOCKED"),
    await expectedSaveRejection(page, "verbatim_namespace", "\\\\?\\C:\\Support\\bundle.zip", "DIAGNOSTIC_PATH_NAMESPACE_BLOCKED"),
    await expectedSaveRejection(page, "wrong_extension", path.join(resultsRoot, `diagnostic-${stamp}.qpls`), "DIAGNOSTIC_EXTENSION_INVALID"),
    await expectedSaveRejection(page, "reserved_device", path.join(resultsRoot, "NUL.zip"), "DIAGNOSTIC_DEVICE_NAME_BLOCKED"),
    await expectedSaveRejection(page, "existing_destination", temporaryExistingPath, "DIAGNOSTIC_DESTINATION_EXISTS"),
  ];
  const existingAfter = await fs.readFile(temporaryExistingPath);
  const existingUnchanged = existingAfter.equals(existingSentinel);
  assert(existingUnchanged, "Existing destination content changed despite no-overwrite contract");
  evidence.checks.destination_rejections = {
    passed: true,
    cases: rejectionCases,
    existing_destination_unchanged: existingUnchanged,
    existing_destination_sha256_before: createHash("sha256").update(existingSentinel).digest("hex"),
    existing_destination_sha256_after: createHash("sha256").update(existingAfter).digest("hex"),
  };

  const observedPageOrigins = [...new Set(evidence.browser_requests.map((row) => row.origin))].sort();
  const externalBrowserRequests = evidence.browser_requests.filter((row) => !allowedPageOrigins.includes(row.origin));
  evidence.checks.browser_network_observation = {
    passed: externalBrowserRequests.length === 0
      && canonicalJson(observedPageOrigins) === canonicalJson(allowedPageOrigins),
    observation: "playwright_page_request_events_during_reload_preview_cancel_navigation_save_and_negative_paths_v1",
    allowed_origins: allowedPageOrigins,
    observed_origins: observedPageOrigins,
    request_count: evidence.browser_requests.length,
    external_request_count: externalBrowserRequests.length,
    external_requests: externalBrowserRequests,
  };
  assert(
    evidence.checks.browser_network_observation.passed
      && canonicalJson(observedPageOrigins) === canonicalJson(allowedPageOrigins),
    "Browser request origins were not exactly the packaged Tauri and IPC origins",
    { observedPageOrigins, externalBrowserRequests },
  );

  evidence.artifacts.diagnostic_zip = await artifactDigest(targetZip);
  evidence.artifacts.live_preview_screenshot = await artifactDigest(livePreviewScreenshot);
  evidence.artifacts.saved_screenshot = await artifactDigest(savedScreenshot);
  evidence.passed = Object.values(evidence.checks).every((check) => check?.passed === true)
    && evidence.console_errors.length === 0 && evidence.failures.length === 0;
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.message : String(error));
} finally {
  if (nativeSaveHelper) nativeSaveHelper.stop();
  if (browser) await browser.close().catch(() => undefined);
  if (temporaryExistingPath) await fs.unlink(temporaryExistingPath).catch(() => undefined);
  evidence.generated_at_utc = new Date().toISOString();
  await fs.writeFile(reportPath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

if (!evidence.passed) {
  console.error(evidence.failures[0] ?? "Diagnostic packaged acceptance did not pass.");
  process.exit(1);
}
console.log(JSON.stringify({ passed: true, report: relativePath(reportPath), zip: evidence.artifacts.diagnostic_zip }, null, 2));
