import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import { chromium } from "playwright";
import {
  RESULTS,
  ROOT,
  ensureDir,
  startPreview,
  stopPreview,
  waitForPreview,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

/**
 * Native-desktop visual acceptance, executed against the production web bundle.
 *
 * This is intentionally a Chromium/Vite-preview harness. It verifies the DOM and
 * responsive renderer used by Tauri, but it cannot prove native window chrome,
 * Windows scaling, file pickers, or engine IPC. Those remain packaged-Tauri QA.
 *
 * Run after building the production bundle:
 *   npm run build
 *   node validation/v247_native_desktop_visual_acceptance.mjs
 */

const port = 57_647;
const baseUrl = `http://127.0.0.1:${port}/`;
const auditOutputId = process.env.QPLS_VISUAL_ACCEPTANCE_OUTPUT_ID?.trim()
  || "v247-native-desktop-visual";
if (!/^[a-z0-9][a-z0-9_-]*$/i.test(auditOutputId)) {
  throw new Error(`QPLS_VISUAL_ACCEPTANCE_OUTPUT_ID is invalid: ${auditOutputId}`);
}
const screenshotDir = path.join(RESULTS, "screens", auditOutputId);
const screenshotPathPrefix = `validation/results/screens/${auditOutputId}/`;
const resultPath = path.join(
  RESULTS,
  process.env.QPLS_VISUAL_ACCEPTANCE_REPORT_NAME?.trim()
    || "v247_native_desktop_visual_acceptance.json",
);

const viewports = [
  { id: "1024x700", width: 1024, height: 700 },
  { id: "1280x720", width: 1280, height: 720 },
  { id: "1440x900", width: 1440, height: 900 },
];

const scale200Viewport = {
  id: "1024x700@200pct-device-scale",
  width: 1024,
  height: 700,
  deviceScaleFactor: 2,
};
const screenshotViewportIds = new Set([
  ...viewports.map((viewport) => viewport.id),
  scale200Viewport.id,
  "1440x900-large-model",
]);

const largeModelTarget = { constructs: 20, indicators: 80 };
const interactionBudgetsMs = {
  fixtureRender: 5_000,
  selection: 750,
  drag: 1_250,
  pan: 1_250,
  zoom: 1_250,
  workspaceRoundTrip: 2_500,
  preflight: 1_000,
  fixtureReopen: 2_500,
};

async function canonicalNativeAnalysisCatalog() {
  const catalogSource = await fs.readFile(path.join(ROOT, "src", "native", "nativeAnalysisCatalog.ts"), "utf8");
  const recipeSource = await fs.readFile(path.join(ROOT, "src", "native", "nativeAnalysisRecipe.ts"), "utf8");
  const calculationModeSource = await fs.readFile(path.join(ROOT, "src", "native", "nativeCalculationMode.ts"), "utf8");
  const catalogMatch = catalogSource.match(/const CATALOG_DRAFTS[^=]*= \[([\s\S]*?)\n\] as const;/);
  if (!catalogMatch) throw new Error("Could not locate the canonical native analysis catalogue declaration.");

  const kinds = [...catalogMatch[1].matchAll(/^[ \t]{4}kind:\s*"([a-z_]+)",\r?$/gm)]
    .map((match) => match[1]);
  if (kinds.length === 0 || new Set(kinds).size !== kinds.length) {
    throw new Error(`The production native execution-adapter order must be non-empty and unique: ${JSON.stringify(kinds)}`);
  }
  const establishedMatch = catalogSource.match(
    /export const NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1\s*=\s*\[([\s\S]*?)\n\] as const satisfies/,
  );
  if (!establishedMatch) throw new Error("Could not locate the established working analysis catalogue declaration.");
  const establishedKinds = [...establishedMatch[1].matchAll(/"([a-z_]+)"/g)].map((match) => match[1]);
  if (establishedKinds.length === 0 || new Set(establishedKinds).size !== establishedKinds.length
    || establishedKinds.some((kind) => !kinds.includes(kind))) {
    throw new Error(`The established working analysis catalogue must be non-empty, unique, and canonical: ${JSON.stringify(establishedKinds)}`);
  }
  const standardSupplementalKinds = [
    "cta_pls",
    "plsc_bootstrap",
    "pls_posthoc_technical_minimum_sample_size",
    "pls_sample_size_power",
  ];
  const expectedKindSet = new Set([...establishedKinds, ...standardSupplementalKinds]);
  if (expectedKindSet.size !== establishedKinds.length + standardSupplementalKinds.length
    || [...expectedKindSet].some((kind) => !kinds.includes(kind))) {
    throw new Error(`The Standard calculation catalogue contains an unknown or duplicate analysis kind: ${JSON.stringify([...expectedKindSet])}`);
  }

  const labelsByKind = new Map(
    [...recipeSource.matchAll(/\{\s*kind:\s*"([a-z_]+)"[^{}]*?\blabel:\s*"([^"]+)"/g)]
      .map((match) => [match[1], match[2]]),
  );
  const predictionLabel = calculationModeSource.match(/export const NATIVE_PREDICTION_METHOD_LABEL\s*=\s*"([^"]+)";/)?.[1];
  const regressionLabel = catalogSource.match(/item\.kind\s*===\s*"regression"\s*\?\s*"([^"]+)"/)?.[1];
  if (!predictionLabel || !regressionLabel) {
    throw new Error("Could not resolve the canonical Prediction or Regression catalogue label.");
  }
  labelsByKind.set("predict", predictionLabel);
  labelsByKind.set("regression", regressionLabel);

  const methods = kinds
    .filter((kind) => expectedKindSet.has(kind))
    .map((kind) => ({ kind, label: labelsByKind.get(kind) ?? null }));
  if (methods.some((method) => !method.label)
    || new Set(methods.map((method) => method.label)).size !== methods.length) {
    throw new Error(`The canonical native analysis catalogue has missing or duplicate labels: ${JSON.stringify(methods)}`);
  }
  return methods;
}

const nativeCalculationMethods = await canonicalNativeAnalysisCatalog();

const nativeNcaScopeNote = "Numeric observed-variable CE-FDH and CR-FDH analysis with observed-range bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and broader ceiling variants are not included.";
const nativePcaScopeNote = "Correlation-matrix PCA of 2 to 50 selected numeric variables with listwise deletion, deterministic component orientation, and no rotation or inferential resampling.";
const nativeOlsScopeNote = "Raw numeric ordinary least squares with an intercept, listwise deletion, HC3 robust standard errors, and fixed two-sided 95% confidence intervals. Optional regression case-resampling reports percentile-primary and conditional BCa inference. Categorical encoding, weights, clusters, generic PLS resampling, logistic regression, and PROCESS models are not included.";
const nativeLogisticScopeNote = "Binary logistic regression with an intercept, raw numeric predictors, listwise deletion, deterministic maximum-likelihood estimation, Wald inference, odds ratios, fitted probabilities, and fixed two-sided 95% confidence intervals. Optional regression case-resampling reports percentile-primary and conditional BCa coefficient and odds-ratio inference. The outcome must be coded exactly 0/1. Multinomial, ordinal, weighted, clustered, penalized, generic PLS resampling, and Firth-corrected models are not included.";
const nativeRegressionBootstrapScopeNote = "10,000 resamples are recommended for final results; 1,000 can be used for exploratory runs. Percentile intervals are primary. BCa is reported when delete-one refits support it, otherwise an explicit reason is shown. Fixed two-sided 95% inference; studentized intervals, one-tailed tests, and custom alpha are excluded. Runtime grows with resamples. Indexed seeded streams make results deterministic and worker-invariant.";
const nativeProcessV2ScopeNote = "Graph-defined observed-variable path analysis with raw listwise-complete OLS equations, HC3 covariance, fixed two-sided 95% Student-t inference, parallel and serial mediation, continuous or exact 0/1 moderation, mixed two-moderator interactions, first- or second-stage moderated mediation, simple slopes, and Johnson-Neyman regions where applicable. This release supports up to 8 selected predictors in graph-role order and one control entered in every equation; the 50-term ceiling is an equation-design safety bound. Continuous product participants are centered within each equation sample. Numbered macros, binary outcomes, weights, clusters, custom alpha or tails, studentized intervals, multiple moderated stages on one indirect path, and three-way interactions on mediated paths are excluded.";
const nativeProcessV2ProbeDisclosure = "Continuous simple-slope and plot probes use the original sample raw mean - SD, mean, and mean + SD; binary probes use original raw 0/1. Resamples and delete-one fits re-center their equations internally while retaining that original raw probe grid.";
const nativeProcessV2BootstrapScopeNote = "10,000 complete-case resamples are recommended for final results; 1,000 can be used for exploratory runs. Percentile intervals are primary. BCa requires every delete-one PROCESS fit; unavailable intervals retain an explicit reason. Fixed two-sided 95% inference; studentized intervals, one-tailed tests, and custom alpha are excluded. Indexed seeded streams are deterministic and worker-invariant.";

const obsoleteRibbonStrings = [
  "Open Setup",
  "Report Wizard",
  "QuickPLS 2.0",
  "v2.43.0 full native wiring",
];

const fakeTitlebarSelector = [
  ".title-bar",
  ".np-titlebar",
  ".nd-titlebar",
  ".app-titlebar",
  ".window-titlebar",
  "[data-tauri-drag-region]",
].join(", ");

const legacyRibbonSelector = [
  ".ribbon",
  ".workflow-strip",
  ".workflow-coach",
  ".np-rail",
  "[data-v216-desktop-shell='title-strip']",
].join(", ");

const evidence = {
  schemaVersion: 2,
  generatedAt: new Date().toISOString(),
  passed: false,
  harness: {
    runtime: "Chromium via Vite production preview",
    actualTauriWindow: false,
    scope: "Tauri frontend DOM, responsive layout, dialogs, keyboard/focus/ARIA contracts, and deterministic sample-result presentation",
    exclusions: [
      "Windows title-bar count and window controls require the packaged Tauri application.",
      "Native file dialogs, print UI, engine IPC, browser-chrome zoom controls, and assistive-technology speech output are not exercised here.",
    ],
    scale200Audit: "Chromium deviceScaleFactor=2 at a 1024x700 CSS viewport; this exercises 200% rendering scale without pretending to be Windows display scaling or browser-chrome zoom.",
  },
  viewports,
  screenshots: [],
  checks: {
    bundleObsoleteStrings: {},
    states: [],
    dialogs: [],
    accessibility: [],
    keyboard: [],
    contextMenus: [],
    workspaceExplorer: [],
    dataImport: [],
    recode: [],
    calculationCatalog: [],
    plsc: [],
    wpls: [],
    gsca: [],
    cca: [],
    ctaPls: [],
    ipma: [],
    cbsem: [],
    nca: [],
    pca: [],
    ols: [],
    logistic: [],
    regressionBootstrap: [],
    processV2: [],
    structuralPathRandomization: [],
    mga: [],
    prediction: [],
    moderationAuthoring: [],
    higherOrderAuthoring: [],
    completedModeration: [],
    mediation: [],
    mediationBootstrapCompact: null,
    scale200Percent: null,
    largeModel: null,
  },
  skipped: [],
  failures: [],
  consoleErrors: [],
};

function recordFailure(id, detail, context = {}) {
  evidence.failures.push({ id, detail, ...context });
}
function recordSkip(id, reason, context = {}) {
  evidence.skipped.push({ id, reason, ...context });
}


async function auditProductionBundle() {
  const assetsDir = path.join(ROOT, "dist", "assets");
  const entries = await fs.readdir(assetsDir, { withFileTypes: true });
  const jsFiles = entries.filter((entry) => entry.isFile() && entry.name.endsWith(".js"));
  if (jsFiles.length === 0) {
    throw new Error("No production JavaScript assets were found. Run `npm run build` before this harness.");
  }

  const contents = await Promise.all(jsFiles.map(async (entry) => ({
    name: entry.name,
    text: await fs.readFile(path.join(assetsDir, entry.name), "utf8"),
  })));

  for (const obsolete of obsoleteRibbonStrings) {
    const files = contents.filter((asset) => asset.text.includes(obsolete)).map((asset) => asset.name);
    evidence.checks.bundleObsoleteStrings[obsolete] = { present: files.length > 0, files };
    if (files.length > 0) {
      recordFailure("obsolete-string-in-production-bundle", `Obsolete ribbon string remains in production JavaScript: ${obsolete}`, { files });
    }
  }
}

async function waitForSmokeApi(page) {
  await page.waitForSelector('.nd-app[data-native-desktop-shell="true"]');
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__?.setView));
}

async function openSmokePage(page, mode = "1") {
  await page.goto(`${baseUrl}?quickpls_smoke=${encodeURIComponent(mode)}`, {
    waitUntil: "domcontentloaded",
    timeout: 45_000,
  });
  await waitForSmokeApi(page);
}

async function setSurface(page, surface) {
  const smokeView = surface === "launcher" ? "home" : surface;
  await page.evaluate((next) => window.__QUICKPLS_SMOKE__?.setView(next), smokeView);
  await page.waitForSelector(`.nd-app[data-surface="${surface}"]`);
  await page.waitForTimeout(100);
}

async function inspectShellState(page, state, viewport) {
  const check = await page.evaluate(({ fakeSelector, ribbonSelector, obsoleteStrings }) => {
    const isVisible = (node) => {
      if (!(node instanceof HTMLElement)) return false;
      const style = window.getComputedStyle(node);
      const rect = node.getBoundingClientRect();
      return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
    };
    const documentWidth = document.documentElement.scrollWidth;
    const bodyWidth = document.body.scrollWidth;
    const viewportWidth = document.documentElement.clientWidth;
    const bodyText = document.body.innerText;
    return {
      documentWidth,
      bodyWidth,
      viewportWidth,
      horizontalOverflow: Math.max(documentWidth, bodyWidth) > viewportWidth + 2,
      visibleHtmlTitlebars: Array.from(document.querySelectorAll(fakeSelector)).filter(isVisible).length,
      visibleLegacyRibbons: Array.from(document.querySelectorAll(ribbonSelector)).filter(isVisible).length,
      obsoleteVisibleStrings: obsoleteStrings.filter((text) => bodyText.includes(text)),
      fabricatedIterationRows: (bodyText.match(/Iteration\s+\d+/gi) ?? []).length,
    };
  }, {
    fakeSelector: fakeTitlebarSelector,
    ribbonSelector: legacyRibbonSelector,
    obsoleteStrings: obsoleteRibbonStrings,
  });

  evidence.checks.states.push({ state, viewport: viewport.id, ...check });
  if (check.horizontalOverflow) recordFailure("horizontal-overflow", `${state} overflows horizontally at ${viewport.id}.`, { state, viewport: viewport.id, check });
  if (check.visibleHtmlTitlebars !== 0) recordFailure("fake-html-titlebar", `${state} exposes ${check.visibleHtmlTitlebars} visible HTML title bar(s) at ${viewport.id}.`, { state, viewport: viewport.id });
  if (check.visibleLegacyRibbons !== 0) recordFailure("legacy-ribbon-visible", `${state} exposes ${check.visibleLegacyRibbons} legacy ribbon/workflow element(s) at ${viewport.id}.`, { state, viewport: viewport.id });
  if (check.obsoleteVisibleStrings.length > 0) recordFailure("obsolete-ribbon-copy-visible", `${state} exposes obsolete ribbon copy at ${viewport.id}.`, { state, viewport: viewport.id, strings: check.obsoleteVisibleStrings });
  if (check.fabricatedIterationRows !== 0) recordFailure("fabricated-iteration-copy", `${state} exposes static iteration text at ${viewport.id}.`, { state, viewport: viewport.id, count: check.fabricatedIterationRows });
}

async function inspectAccessibility(page, state, viewport) {
  const check = await page.evaluate(() => {
    const isVisible = (node) => {
      if (!(node instanceof HTMLElement)) return false;
      const style = window.getComputedStyle(node);
      const rect = node.getBoundingClientRect();
      return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
    };
    const text = (value) => (value ?? "").replace(/\s+/g, " ").trim();
    const labelledByName = (node) => text((node.getAttribute("aria-labelledby") ?? "")
      .split(/\s+/)
      .filter(Boolean)
      .map((id) => document.getElementById(id)?.textContent ?? "")
      .join(" "));
    const accessibleName = (node) => {
      const ariaLabel = text(node.getAttribute("aria-label"));
      if (ariaLabel) return ariaLabel;
      const labelled = labelledByName(node);
      if (labelled) return labelled;
      if (node instanceof HTMLInputElement || node instanceof HTMLSelectElement || node instanceof HTMLTextAreaElement) {
        const labels = text(Array.from(node.labels ?? []).map((label) => label.textContent ?? "").join(" "));
        if (labels) return labels;
      }
      if (node instanceof HTMLImageElement) return text(node.alt);
      return text(node.textContent) || text(node.getAttribute("title"));
    };

    const interactiveSelector = [
      "button",
      "a[href]",
      "input:not([type='hidden'])",
      "select",
      "textarea",
      "[role='button']",
      "[role='menuitem']",
      "[role='tab']",
    ].join(",");
    const visibleInteractive = Array.from(document.querySelectorAll(interactiveSelector)).filter(isVisible);
    const unnamedInteractive = visibleInteractive
      .filter((node) => !accessibleName(node))
      .map((node) => ({ tag: node.tagName.toLowerCase(), role: node.getAttribute("role"), className: node.className, snippet: text(node.outerHTML).slice(0, 180) }));

    const brokenLabelledBy = Array.from(document.querySelectorAll("[aria-labelledby]"))
      .filter(isVisible)
      .map((node) => ({
        element: `${node.tagName.toLowerCase()}.${typeof node.className === "string" ? node.className : ""}`,
        ids: (node.getAttribute("aria-labelledby") ?? "").split(/\s+/).filter(Boolean),
      }))
      .filter((item) => item.ids.length === 0 || item.ids.some((id) => !document.getElementById(id)));

    const idCounts = new Map();
    for (const node of document.querySelectorAll("[id]")) idCounts.set(node.id, (idCounts.get(node.id) ?? 0) + 1);
    const duplicateIds = Array.from(idCounts).filter(([, count]) => count > 1).map(([id, count]) => ({ id, count }));
    const positiveTabIndexes = Array.from(document.querySelectorAll("[tabindex]"))
      .filter(isVisible)
      .filter((node) => Number(node.getAttribute("tabindex")) > 0)
      .map((node) => ({ tag: node.tagName.toLowerCase(), tabIndex: node.getAttribute("tabindex"), className: node.className }));

    const workspace = document.getElementById("nd-main");
    const appMenu = document.querySelector("nav[aria-label='Application menu']");
    const commandToolbar = document.querySelector("[role='toolbar'][aria-label]");
    return {
      visibleInteractiveCount: visibleInteractive.length,
      unnamedInteractive,
      brokenLabelledBy,
      duplicateIds,
      positiveTabIndexes,
      skipLinkPresent: Boolean(document.querySelector("a.nd-skip-link[href='#nd-main']")),
      workspaceTargetPresent: Boolean(workspace),
      workspaceIsMainLandmark: Boolean(workspace?.matches("main, [role='main']")),
      namedApplicationMenuPresent: Boolean(appMenu),
      namedCommandToolbarPresent: Boolean(commandToolbar),
    };
  });

  const entry = { state, viewport: viewport.id, ...check };
  evidence.checks.accessibility.push(entry);
  if (check.unnamedInteractive.length > 0) recordFailure("unnamed-visible-control", `${state} has ${check.unnamedInteractive.length} visible control(s) without an accessible name at ${viewport.id}.`, entry);
  if (check.brokenLabelledBy.length > 0) recordFailure("broken-aria-labelledby", `${state} has broken aria-labelledby reference(s) at ${viewport.id}.`, entry);
  if (check.duplicateIds.length > 0) recordFailure("duplicate-dom-id", `${state} has duplicate DOM id(s) at ${viewport.id}.`, entry);
  if (check.positiveTabIndexes.length > 0) recordFailure("positive-tabindex", `${state} uses positive tabindex values at ${viewport.id}.`, entry);
  if (!check.skipLinkPresent || !check.workspaceTargetPresent) recordFailure("skip-link-contract", `${state} does not expose a valid skip link and workspace target at ${viewport.id}.`, entry);
  if (!check.workspaceIsMainLandmark) recordFailure("workspace-main-landmark", `${state} workspace is not exposed as a main landmark at ${viewport.id}.`, entry);
  if (!check.namedApplicationMenuPresent || !check.namedCommandToolbarPresent) recordFailure("shell-landmark-names", `${state} is missing its named application menu or command toolbar at ${viewport.id}.`, entry);
}
async function inspectDialog(page, dialog, viewport) {
  const selector = `.nd-dialog-${dialog}[role="dialog"]`;
  const locator = page.locator(selector).filter({ visible: true });
  const count = await locator.count();
  let ariaModal = null;
  let labelledBy = null;
  let accessibleName = "";
  if (count === 1) {
    ariaModal = await locator.getAttribute("aria-modal");
    labelledBy = await locator.getAttribute("aria-labelledby");
    accessibleName = labelledBy
      ? (await page.locator(`#${labelledBy}`).first().textContent())?.trim() ?? ""
      : (await locator.getAttribute("aria-label"))?.trim() ?? "";
  }
  const check = { dialog, viewport: viewport.id, count, ariaModal, labelledBy, accessibleName };
  evidence.checks.dialogs.push(check);
  if (count !== 1 || ariaModal !== "true" || !accessibleName) {
    recordFailure("dialog-accessibility-contract", `${dialog} dialog lacks a unique, named, modal dialog role at ${viewport.id}.`, check);
  }
}

async function auditImportDataDialog(page, viewport, sequence) {
  const trigger = page.locator(".nd-commandbar button").filter({ hasText: /Import Data/i }).first();
  const dialog = page.locator('.nd-dialog-import-data[role="dialog"]');
  const fileChooserEvents = [];
  const recordFileChooser = () => fileChooserEvents.push("filechooser");
  page.on("filechooser", recordFileChooser);

  let dialogOpened = false;
  let covarianceSelected = false;
  let sampleSizeVisible = false;
  let belowMinimumRejected = false;
  let nativeValidationMessage = "";
  let validationAlert = "";
  let dialogClosed = false;
  let focusRestored = false;

  try {
    if (await trigger.count() !== 1) {
      recordFailure("import-data-dialog-trigger-missing", `No Import Data command was rendered in the Data toolbar at ${viewport.id}.`);
      return;
    }

    await trigger.click();
    dialogOpened = await dialog.isVisible().catch(() => false);
    if (!dialogOpened) {
      recordFailure("import-data-dialog-open", `The Import Data command did not open its native setup dialog at ${viewport.id}.`);
      return;
    }

    const covariance = dialog.getByRole("radio", { name: /Covariance matrix/i });
    await covariance.check();
    covarianceSelected = await covariance.isChecked();

    const sampleSize = dialog.getByLabel(/Study sample size/i);
    sampleSizeVisible = await sampleSize.isVisible().catch(() => false);
    if (sampleSizeVisible) {
      await sampleSize.fill("1");
      const nativeValidation = await sampleSize.evaluate((node) => ({
        valid: node instanceof HTMLInputElement ? node.validity.valid : true,
        rangeUnderflow: node instanceof HTMLInputElement ? node.validity.rangeUnderflow : false,
        message: node instanceof HTMLInputElement ? node.validationMessage : "",
      }));
      belowMinimumRejected = !nativeValidation.valid && nativeValidation.rangeUnderflow;
      nativeValidationMessage = nativeValidation.message;
      await sampleSize.fill("");
    }

    await dialog.getByRole("button", { name: /Choose File/i }).click();
    const alert = dialog.locator('.nd-form-error[role="alert"]');
    await alert.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    validationAlert = (await alert.textContent().catch(() => ""))?.replace(/\s+/g, " ").trim() ?? "";

    await capture(page, "import-data-dialog", sequence, viewport, { dialog: "import-data" });

    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
    await page.waitForTimeout(50);
    dialogClosed = await dialog.isHidden().catch(() => true);
    focusRestored = await trigger.evaluate((node) => document.activeElement === node);
  } finally {
    page.off("filechooser", recordFileChooser);
    if (await dialog.isVisible().catch(() => false)) {
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click().catch(() => null);
      await dialog.waitFor({ state: "hidden" }).catch(() => null);
    }

    const check = {
      viewport: viewport.id,
      dialogOpened,
      covarianceSelected,
      sampleSizeVisible,
      belowMinimumSampleSize: 1,
      belowMinimumRejected,
      nativeValidationMessage,
      submittedInvalidSampleSize: "blank",
      validationAlert,
      validationBlockedFileChooser: fileChooserEvents.length === 0,
      fileChooserEvents,
      dialogClosed,
      focusRestored,
      nativeRuntimePresent: await page.evaluate(() => Boolean(window.__TAURI_INTERNALS__)),
    };
    evidence.checks.dataImport.push(check);
    if (!dialogOpened || !covarianceSelected || !sampleSizeVisible) {
      recordFailure("import-data-covariance-setup", `The covariance import setup was incomplete at ${viewport.id}.`, check);
    }
    if (!belowMinimumRejected) {
      recordFailure("import-data-sample-size-minimum", `The Study sample size control accepted a covariance sample size below 2 at ${viewport.id}.`, check);
    }
    if (!/sample size/i.test(validationAlert) || !/at least 2/i.test(validationAlert)) {
      recordFailure("import-data-sample-size-validation", `Covariance import did not show the expected sample-size validation at ${viewport.id}.`, check);
    }
    if (fileChooserEvents.length !== 0) {
      recordFailure("import-data-validation-opened-picker", `Invalid covariance setup opened a file chooser at ${viewport.id}.`, check);
    }
    if (dialogOpened && (!dialogClosed || !focusRestored)) {
      recordFailure("import-data-cancel-focus", `Cancel did not close Import Data and restore toolbar focus at ${viewport.id}.`, check);
    }
  }
}

async function auditRecodeDialog(page, viewport, sequence) {
  const expectedSource = "COMP1";
  const expectedBrowserExplanation = "Recode creates an immutable dataset version and is available only in the installed Windows app. Browser preview cannot write dataset versions.";
  const sourceVariable = page.locator(`.nd-variable-list button[data-native-variable="${expectedSource}"]`);
  const trigger = page.locator(".nd-commandbar button").filter({ hasText: /Recode Variable/i });
  const dialog = page.locator('.nd-dialog-recode-data[role="dialog"]');

  let sourceVariableCount = 0;
  let sourceSelected = false;
  let toolbarTriggerCount = 0;
  let toolbarTriggerEnabled = false;
  let navigatorRecodeButtonCount = 0;
  let dialogOpened = false;
  let sourceControlCount = 0;
  let sourceControlValue = "";
  let sourceControlDisabled = false;
  let browserExplanation = "";
  let createButtonCount = 0;
  let createButtonDisabled = false;
  let ariaBusy = null;
  let busyIndicatorCount = 0;
  let progressCount = 0;
  let creatingVersionTextCount = 0;
  let horizontalOverflow = null;
  let dialogClosed = false;
  let focusRestored = false;

  try {
    sourceVariableCount = await sourceVariable.count();
    if (sourceVariableCount === 1) {
      await sourceVariable.click();
      await page.waitForFunction((column) => document.querySelector(`.nd-variable-list button[data-native-variable="${column}"]`)?.classList.contains("active"), expectedSource, { timeout: 1_000 }).catch(() => null);
      sourceSelected = await sourceVariable.evaluate((node) => node.classList.contains("active"));
    }

    toolbarTriggerCount = await trigger.count();
    navigatorRecodeButtonCount = await page.locator(".nd-navigator button").filter({ hasText: /Recode/i }).count();
    if (toolbarTriggerCount !== 1) {
      recordFailure("recode-toolbar-trigger", `Expected exactly one registry Recode Variable command in the Data toolbar at ${viewport.id}.`, {
        viewport: viewport.id,
        toolbarTriggerCount,
        navigatorRecodeButtonCount,
      });
      return;
    }
    toolbarTriggerEnabled = await trigger.isEnabled();
    if (!toolbarTriggerEnabled) {
      recordFailure("recode-toolbar-trigger-disabled", `The selected COMP1 variable did not enable the registry Recode Variable command at ${viewport.id}.`);
      return;
    }

    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    dialogOpened = await dialog.isVisible().catch(() => false);
    if (!dialogOpened) {
      recordFailure("recode-dialog-open", `The registry Recode Variable command did not open its dialog at ${viewport.id}.`);
      return;
    }

    const sourceControl = dialog.getByLabel("Source indicator", { exact: true });
    sourceControlCount = await sourceControl.count();
    if (sourceControlCount === 1) {
      sourceControlValue = await sourceControl.inputValue();
      sourceControlDisabled = await sourceControl.isDisabled();
    }
    browserExplanation = (await dialog.locator('.nd-recode-disabled[role="status"]').textContent().catch(() => ""))?.replace(/\s+/g, " ").trim() ?? "";

    const createButton = dialog.getByRole("button", { name: "Create Recode", exact: true });
    createButtonCount = await createButton.count();
    if (createButtonCount === 1) createButtonDisabled = await createButton.isDisabled();

    const recodeForm = dialog.locator("form.nd-recode-dialog");
    ariaBusy = await recodeForm.getAttribute("aria-busy");
    busyIndicatorCount = await dialog.locator('.nd-spin, [aria-busy="true"]').count();
    progressCount = await dialog.locator('progress, [role="progressbar"]').count();
    creatingVersionTextCount = await dialog.getByText("Creating version...", { exact: true }).count();
    horizontalOverflow = await dialog.evaluate((node) => {
      const rect = node.getBoundingClientRect();
      const documentWidth = document.documentElement.scrollWidth;
      const bodyWidth = document.body.scrollWidth;
      const viewportWidth = document.documentElement.clientWidth;
      return {
        dialogScrollWidth: node.scrollWidth,
        dialogClientWidth: node.clientWidth,
        dialogOutsideViewport: rect.left < -2 || rect.right > window.innerWidth + 2,
        dialogContentOverflow: node.scrollWidth > node.clientWidth + 2,
        pageOverflow: Math.max(documentWidth, bodyWidth) > viewportWidth + 2,
      };
    });

    await capture(page, "recode-dialog", sequence, viewport, { dialog: "recode-data" });

    const closeButton = dialog.getByRole("button", { name: "Close dialog", exact: true });
    if (await closeButton.count() === 1) await closeButton.click();
    await dialog.waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
    dialogClosed = await dialog.isHidden().catch(() => true);
    const triggerHandle = await trigger.elementHandle();
    if (triggerHandle) {
      await page.waitForFunction((node) => document.activeElement === node, triggerHandle, { timeout: 1_000 }).catch(() => null);
      focusRestored = await trigger.evaluate((node) => document.activeElement === node);
    }
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click().catch(() => null);
      await dialog.waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
    }

    const correctSource = sourceVariableCount === 1
      && sourceSelected
      && sourceControlCount === 1
      && sourceControlValue === expectedSource
      && sourceControlDisabled;
    const browserNativeOnlyTruthful = browserExplanation === expectedBrowserExplanation
      && createButtonCount === 1
      && createButtonDisabled;
    const noBusyProgress = ariaBusy === "false"
      && busyIndicatorCount === 0
      && progressCount === 0
      && creatingVersionTextCount === 0;
    const noHorizontalOverflow = horizontalOverflow != null
      && !horizontalOverflow.dialogOutsideViewport
      && !horizontalOverflow.dialogContentOverflow
      && !horizontalOverflow.pageOverflow;
    const check = {
      viewport: viewport.id,
      expectedSource,
      sourceVariableCount,
      sourceSelected,
      toolbarTriggerCount,
      toolbarTriggerEnabled,
      navigatorRecodeButtonCount,
      noDuplicateRecodeButton: toolbarTriggerCount === 1 && navigatorRecodeButtonCount === 0,
      dialogOpened,
      sourceControlCount,
      sourceControlValue,
      sourceControlDisabled,
      correctSource,
      browserExplanation,
      expectedBrowserExplanation,
      createButtonCount,
      createButtonDisabled,
      browserNativeOnlyTruthful,
      ariaBusy,
      busyIndicatorCount,
      progressCount,
      creatingVersionTextCount,
      noBusyProgress,
      horizontalOverflow,
      noHorizontalOverflow,
      dialogClosed,
      focusRestored,
      nativeRuntimePresent: await page.evaluate(() => Boolean(window.__TAURI_INTERNALS__)),
      nativeMutationInvoked: false,
    };
    evidence.checks.recode.push(check);

    if (!correctSource) {
      recordFailure("recode-source-contract", `Recode did not preserve selected COMP1 as its disabled source indicator at ${viewport.id}.`, check);
    }
    if (toolbarTriggerCount !== 1 || navigatorRecodeButtonCount !== 0) {
      recordFailure("recode-command-duplication", `Recode was not represented by exactly one Data-toolbar registry command at ${viewport.id}.`, check);
    }
    if (!dialogOpened || !browserNativeOnlyTruthful) {
      recordFailure("recode-browser-native-only-contract", `Browser Recode did not truthfully disclose native-only version creation with Create disabled at ${viewport.id}.`, check);
    }
    if (!noBusyProgress) {
      recordFailure("recode-fabricated-busy-state", `Idle browser Recode exposed a busy or progress state at ${viewport.id}.`, check);
    }
    if (!noHorizontalOverflow) {
      recordFailure("recode-dialog-horizontal-overflow", `Recode overflowed horizontally at ${viewport.id}.`, check);
    }
    if (!dialogClosed || !focusRestored) {
      recordFailure("recode-dialog-close-focus", `Close did not dismiss Recode and restore registry-toolbar focus at ${viewport.id}.`, check);
    }
  }
}

async function capture(page, state, sequence, viewport, { dialog = null } = {}) {
  if (typeof state !== "string" || !/^[a-z0-9][a-z0-9-]*$/.test(state)
    || !screenshotViewportIds.has(viewport?.id)) {
    throw new Error(`Invalid screenshot viewport/state identity: ${JSON.stringify({ viewport: viewport?.id, state })}`);
  }
  await inspectShellState(page, state, viewport);
  await inspectAccessibility(page, state, viewport);
  if (dialog) await inspectDialog(page, dialog, viewport);
  const filename = `${String(sequence).padStart(2, "0")}-${state}-${viewport.id}.png`;
  const screenshot = path.join(screenshotDir, filename);
  await page.screenshot({ path: screenshot, fullPage: false, animations: "disabled" });
  const [stat, bytes] = await Promise.all([fs.stat(screenshot), fs.readFile(screenshot)]);
  if (!stat.isFile() || stat.size <= 0 || stat.size !== bytes.byteLength) {
    throw new Error(`Screenshot write did not produce a stable non-empty file: ${screenshot}`);
  }
  const relativePath = path.relative(ROOT, screenshot).replaceAll("\\", "/");
  if (!relativePath.startsWith(screenshotPathPrefix) || path.basename(relativePath) !== filename) {
    throw new Error(`Screenshot escaped the dedicated visual evidence directory: ${relativePath}`);
  }
  evidence.screenshots.push({
    path: relativePath,
    size: stat.size,
    sha256: createHash("sha256").update(bytes).digest("hex"),
    viewport: viewport.id,
    state,
  });
}

const mediationResultTitles = [
  "Direct effects",
  "Specific indirect effects",
  "Total indirect effects",
  "Total effects",
  "Aggregate mediation effects bootstrap inference",
];

async function auditCompletedMediationResults(page, viewport, sequence) {
  const mediationGroup = page.getByRole("treeitem", { name: "Mediation", exact: true });
  await mediationGroup.waitFor({ state: "visible", timeout: 15_000 });
  if (await mediationGroup.getAttribute("aria-expanded") === "false") await mediationGroup.click();

  const rowCounts = {};
  for (const title of mediationResultTitles) {
    const item = page.getByRole("treeitem", { name: title, exact: true });
    await item.waitFor({ state: "visible", timeout: 15_000 });
    await item.click();
    await page.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible", timeout: 15_000 });
    rowCounts[title] = await page.locator(".nd-result-table tbody tr").count();
  }

  // Keep the visual evidence on the path-specific output rather than a generic
  // effects table. The completed smoke run is the checked-in deterministic
  // Corporate Reputation result; this harness never injects presentation rows.
  await page.getByRole("treeitem", { name: "Specific indirect effects", exact: true }).click();
  await page.getByRole("heading", { name: "Specific indirect effects", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  const selectedTableText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  const check = {
    viewport: viewport.id,
    source: "completedSamplePlsRun",
    runId: await page.locator(".nd-run-select select").inputValue(),
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    groupTitle: (await mediationGroup.textContent())?.trim() ?? "",
    tableTitles: mediationResultTitles,
    rowCounts,
    selectedTable: "Specific indirect effects",
    selectedTableText,
  };
  evidence.checks.mediation.push(check);

  if (check.runId !== "v11-smoke-completed-pls") {
    recordFailure("mediation-result-fixture-provenance", `The mediation screenshot at ${viewport.id} was not sourced from the checked-in completed smoke run.`, check);
  }
  if (check.runLabel !== "PLS-SEM Bootstrapping run") {
    recordFailure("mediation-result-fixture-label", `The embedded-bootstrap fixture was not truthfully labelled as PLS-SEM Bootstrapping at ${viewport.id}.`, check);
  }
  const emptyTables = mediationResultTitles.filter((title) => !rowCounts[title]);
  if (emptyTables.length) {
    recordFailure("mediation-result-tables-empty", `The truthful completed fixture exposed empty mediation table(s) at ${viewport.id}: ${emptyTables.join(", ")}.`, check);
  }
  if (!selectedTableText) {
    recordFailure("mediation-specific-indirect-empty", `Specific indirect effects had no truthful result rows at ${viewport.id}.`, check);
  }
  await capture(page, "mediation-results", sequence, viewport);
  if (viewport.id === "1024x700") {
    await auditCompactMediationBootstrapInference(page, viewport, sequence + 1);
  }
}

async function auditCompletedModerationResultsIfAvailable(page, viewport, sequence) {
  const group = page.getByRole("treeitem", { name: "Moderation", exact: true });
  const groupCount = await group.count();
  const runId = await page.locator(".nd-run-select select").inputValue();
  const runLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "";
  if (groupCount !== 1) {
    const check = {
      viewport: viewport.id,
      available: false,
      groupCount,
      runId,
      runLabel,
      reason: "The production browser smoke API exposes the checked-in Corporate Reputation mediation/bootstrap run, which has no moderation payload.",
    };
    evidence.checks.completedModeration.push(check);
    recordSkip("completed-moderation-browser-fixture", check.reason, {
      viewport: viewport.id,
      runId,
      runLabel,
      requiredNativeFollowUp: "Run the tracked moderation_reference_base.csv workflow in the packaged Tauri app and verify genuine persisted results.",
    });
    return;
  }

  if (await group.getAttribute("aria-expanded") === "false") await group.click();
  const baseTitles = ["Moderation effects", "Simple slope analysis"];
  const bootstrapTitle = "Interaction effect bootstrap inference";
  const bootstrapItemCount = await page.getByRole("treeitem", { name: bootstrapTitle, exact: true }).count();
  const requiredTitles = [...baseTitles, ...(bootstrapItemCount === 1 ? [bootstrapTitle] : [])];
  const rowCounts = {};
  const tableText = {};
  for (const title of requiredTitles) {
    const item = page.getByRole("treeitem", { name: title, exact: true });
    await item.click();
    await page.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible", timeout: 2_000 });
    rowCounts[title] = await page.locator(".nd-result-table tbody tr").count();
    tableText[title] = compactCalculationText(await page.locator(".nd-result-table tbody").textContent());
  }

  await page.getByRole("treeitem", { name: "Simple slope analysis", exact: true }).click();
  const plot = page.locator(".nd-moderation-plot");
  const check = {
    viewport: viewport.id,
    available: true,
    groupCount,
    runId,
    runLabel,
    requiredTitles,
    rowCounts,
    tableText,
    bootstrapItemCount,
    conditionalEffectPlotCount: await plot.count(),
    accessiblePlotCount: await plot.locator('svg[role="img"][aria-labelledby]').count(),
    plottedPoints: await plot.locator("circle").count(),
  };
  evidence.checks.completedModeration.push(check);

  if (requiredTitles.some((title) => !rowCounts[title] || !tableText[title] || /\bN\/A\b/i.test(tableText[title]))) {
    recordFailure("completed-moderation-table-contract", `The truthful completed moderation fixture exposed an empty or placeholder result table at ${viewport.id}.`, check);
  }
  if (check.conditionalEffectPlotCount !== 1 || check.accessiblePlotCount !== 1 || check.plottedPoints < 3) {
    recordFailure("completed-moderation-plot-contract", `The truthful completed moderation fixture did not expose one accessible conditional-effect plot backed by reported slope points at ${viewport.id}.`, check);
  }
  await capture(page, "moderation-results", sequence, viewport);
}

async function auditCompactMediationBootstrapInference(page, viewport, sequence) {
  const title = "Aggregate mediation effects bootstrap inference";
  await page.getByRole("treeitem", { name: title, exact: true }).click();
  await page.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible", timeout: 15_000 });

  const tableScroll = page.locator(".nd-result-table-view > .nd-table-scroll");
  const tableScrollCount = await tableScroll.count();
  let layout = {
    tablePresent: false,
    role: null,
    labelledBy: null,
    labelledByHeading: false,
    activeCellCount: 0,
    finalActiveCellCount: 0,
    initialActiveColumn: null,
    finalActiveColumn: null,
    overflowX: null,
    clientWidth: 0,
    scrollWidth: 0,
    initialScrollLeft: 0,
    finalScrollLeft: 0,
    activeCellFocused: false,
    activeColumnMoved: false,
    keyboardScrollMoved: false,
  };

  if (tableScrollCount === 1) {
    await tableScroll.evaluate((element) => { element.scrollLeft = 0; });
    const activeCell = tableScroll.locator('[role="gridcell"][tabindex="0"][aria-selected="true"]');
    const activeCellCount = await activeCell.count();
    if (activeCellCount === 1) await activeCell.focus();
    const before = await tableScroll.evaluate((element) => {
      const style = window.getComputedStyle(element);
      const labelledBy = element.getAttribute("aria-labelledby");
      const active = element.querySelector('[role="gridcell"][tabindex="0"][aria-selected="true"]');
      return {
        tablePresent: Boolean(element.querySelector("table.nd-result-table")),
        role: element.getAttribute("role"),
        labelledBy,
        labelledByHeading: Boolean(labelledBy && document.getElementById(labelledBy)?.textContent?.trim() === "Aggregate mediation effects bootstrap inference"),
        activeCellCount: element.querySelectorAll('[role="gridcell"][tabindex="0"][aria-selected="true"]').length,
        initialActiveColumn: active?.getAttribute("aria-colindex") ?? active?.getAttribute("data-native-grid-column"),
        overflowX: style.overflowX,
        clientWidth: element.clientWidth,
        scrollWidth: element.scrollWidth,
        initialScrollLeft: element.scrollLeft,
        activeCellFocused: document.activeElement === active,
      };
    });
    for (let index = 0; index < 8; index += 1) await page.keyboard.press("ArrowRight");
    await page.waitForTimeout(150);
    const after = await tableScroll.evaluate((element) => {
      const active = element.querySelector('[role="gridcell"][tabindex="0"][aria-selected="true"]');
      return {
        finalActiveCellCount: element.querySelectorAll('[role="gridcell"][tabindex="0"][aria-selected="true"]').length,
        finalActiveColumn: active?.getAttribute("aria-colindex") ?? active?.getAttribute("data-native-grid-column"),
        finalScrollLeft: element.scrollLeft,
        finalActiveCellFocused: document.activeElement === active,
      };
    });
    layout = {
      ...layout,
      ...before,
      ...after,
      activeCellFocused: before.activeCellFocused && after.finalActiveCellFocused,
      activeColumnMoved: Number(after.finalActiveColumn) > Number(before.initialActiveColumn),
      keyboardScrollMoved: after.finalScrollLeft > before.initialScrollLeft,
    };
  }

  const pageLayout = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    bodyWidth: document.body.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
    pageHorizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) > document.documentElement.clientWidth + 2,
  }));
  const check = {
    viewport: viewport.id,
    title,
    tableScrollCount,
    rows: await page.locator(".nd-result-table tbody tr").count(),
    columns: await page.locator(".nd-result-table thead th").count(),
    tableBodyText: (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "",
    internalHorizontalOverflow: layout.scrollWidth > layout.clientWidth + 1,
    layout,
    pageLayout,
  };
  evidence.checks.mediationBootstrapCompact = check;

  if (tableScrollCount !== 1 || !layout.tablePresent || layout.role !== "region" || !layout.labelledByHeading || layout.activeCellCount !== 1 || layout.finalActiveCellCount !== 1) {
    recordFailure("mediation-bootstrap-internal-grid-contract", "The 1024x700 mediation bootstrap table did not expose one heading-labelled region with exactly one roving active gridcell.", check);
  }
  if (check.rows === 0 || check.columns < 8 || !check.tableBodyText.includes("Total indirect effect (aggregate)")) {
    recordFailure("mediation-bootstrap-wide-table-content", "The compact mediation bootstrap evidence did not contain the genuine wide aggregate table.", check);
  }
  if (!check.internalHorizontalOverflow || !["auto", "scroll"].includes(layout.overflowX ?? "")) {
    recordFailure("mediation-bootstrap-internal-overflow", "The wide mediation bootstrap table did not remain horizontally scrollable inside its result pane at 1024x700.", check);
  }
  if (pageLayout.pageHorizontalOverflow) {
    recordFailure("mediation-bootstrap-page-overflow", "The mediation bootstrap table caused page-level horizontal overflow at 1024x700.", check);
  }
  if (!layout.activeCellFocused || !layout.activeColumnMoved || (check.internalHorizontalOverflow && !layout.keyboardScrollMoved)) {
    recordFailure("mediation-bootstrap-keyboard-grid-navigation", "ArrowRight did not move the roving active gridcell and scroll the compact mediation table when needed.", check);
  }
  await capture(page, "mediation-bootstrap-inference", sequence, viewport);
}
async function auditKeyboardAndFocus(page, viewport) {
  await setSurface(page, "launcher");
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    window.location.hash = "";
  });
  await page.keyboard.press("Tab");
  const skipLink = await page.evaluate(() => {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement)) return { focused: false, visible: false, focusVisible: false, active: "" };
    const rect = active.getBoundingClientRect();
    return {
      focused: active.matches("a.nd-skip-link[href='#nd-main']"),
      visible: rect.top >= 0 && rect.left >= 0 && rect.bottom <= window.innerHeight && rect.right <= window.innerWidth,
      focusVisible: active.matches(":focus-visible"),
      active: `${active.tagName.toLowerCase()}.${active.className}`,
    };
  });
  await page.keyboard.press("Enter");
  await page.waitForTimeout(50);
  const skipMovedFocus = await page.evaluate(() => document.activeElement?.id === "nd-main");

  await page.keyboard.press("Control+n");
  const newProjectDialog = page.locator('.nd-dialog-new-project[role="dialog"]');
  const shortcutOpenedDialog = await newProjectDialog.isVisible().catch(() => false);
  let initialDialogFocusInside = false;
  let forwardTrap = false;
  let reverseTrap = false;
  let escapeClosedDialog = false;
  let focusRestored = false;
  let focusableCount = 0;
  if (shortcutOpenedDialog) {
    initialDialogFocusInside = await page.evaluate(() => Boolean(document.activeElement?.closest('.nd-dialog-new-project[role="dialog"]')));
    const focusables = newProjectDialog.locator('button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [href], [tabindex]:not([tabindex="-1"])');
    focusableCount = await focusables.count();
    if (focusableCount > 1) {
      await focusables.nth(focusableCount - 1).focus();
      await page.keyboard.press("Tab");
      forwardTrap = await focusables.first().evaluate((node) => document.activeElement === node);
      await focusables.first().focus();
      await page.keyboard.press("Shift+Tab");
      reverseTrap = await focusables.nth(focusableCount - 1).evaluate((node) => document.activeElement === node);
    }
    await page.keyboard.press("Escape");
    escapeClosedDialog = await newProjectDialog.isHidden().catch(() => true);
    await page.waitForFunction(() => document.activeElement?.id === "nd-main", undefined, { timeout: 1_000 }).catch(() => null);
    focusRestored = await page.evaluate(() => document.activeElement?.id === "nd-main");
  }

  const check = {
    viewport: viewport.id,
    skipLink,
    skipMovedFocus,
    shortcut: "Ctrl+N",
    shortcutOpenedDialog,
    initialDialogFocusInside,
    focusableCount,
    forwardTrap,
    reverseTrap,
    escapeClosedDialog,
    focusRestored,
  };
  evidence.checks.keyboard.push(check);
  if (!skipLink.focused || !skipLink.visible || !skipLink.focusVisible || !skipMovedFocus) {
    recordFailure("skip-link-keyboard-focus", `The skip link keyboard contract failed at ${viewport.id}.`, check);
  }
  if (!shortcutOpenedDialog || !initialDialogFocusInside) {
    recordFailure("keyboard-shortcut-dialog-focus", `Ctrl+N did not open and focus the New Project dialog at ${viewport.id}.`, check);
  }
  if (focusableCount <= 1 || !forwardTrap || !reverseTrap) {
    recordFailure("dialog-focus-trap", `The New Project dialog did not retain forward and reverse tab focus at ${viewport.id}.`, check);
  }
  if (!escapeClosedDialog || !focusRestored) {
    recordFailure("dialog-escape-focus-restore", `Escape did not close the dialog and restore prior focus at ${viewport.id}.`, check);
  }
}

async function auditWorkspaceExplorer(page, viewport, sequence) {
  const tree = page.locator('.nd-project-tree[role="tree"]');
  const treeItems = tree.locator('[role="treeitem"]');
  const modelItem = tree.locator('[role="treeitem"][data-kind="model"]').first();
  const modelsItem = tree.locator('[role="treeitem"][data-kind="models"]');
  const contextMenu = page.locator('.nd-explorer-context-menu[role="menu"]');
  const renameDialog = page.locator('.nd-dialog-explorer[role="dialog"]');

  const initial = await page.evaluate(() => {
    const visible = (node) => {
      if (!(node instanceof HTMLElement)) return false;
      const style = getComputedStyle(node);
      const rect = node.getBoundingClientRect();
      return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
    };
    const trees = Array.from(document.querySelectorAll('.nd-project-tree[role="tree"]')).filter(visible);
    const items = trees.flatMap((candidate) => Array.from(candidate.querySelectorAll('[role="treeitem"]')));
    const workspace = document.querySelector('.nd-project-workspace');
    const explorer = document.querySelector('.nd-workspace-explorer');
    const detail = document.querySelector('.nd-explorer-detail');
    const layout = [workspace, explorer, detail].map((node) => node instanceof HTMLElement ? {
      className: node.className,
      clientWidth: node.clientWidth,
      scrollWidth: node.scrollWidth,
      clientHeight: node.clientHeight,
      scrollHeight: node.scrollHeight,
      horizontalOverflow: node.scrollWidth > node.clientWidth + 2,
    } : null);
    return {
      treeCount: trees.length,
      itemCount: items.length,
      kinds: items.map((node) => node.getAttribute("data-kind")),
      labels: items.map((node) => (node.querySelector('.nd-tree-label')?.textContent ?? "").replace(/\s+/g, " ").trim()),
      levels: items.map((node) => Number(node.getAttribute("aria-level"))),
      rovingTabStopCount: items.filter((node) => node.getAttribute("tabindex") === "0").length,
      selectedCount: items.filter((node) => node.getAttribute("aria-selected") === "true").length,
      documentHorizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) > document.documentElement.clientWidth + 2,
      layout,
    };
  });

  const keyboard = [];
  const activeTreeItem = () => page.evaluate(() => {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement) || active.getAttribute("role") !== "treeitem") return null;
    return {
      kind: active.dataset.kind ?? null,
      label: (active.querySelector('.nd-tree-label')?.textContent ?? "").replace(/\s+/g, " ").trim(),
      expanded: active.getAttribute("aria-expanded"),
    };
  });
  const pressAndRecord = async (key) => {
    await page.keyboard.press(key);
    await page.waitForTimeout(30);
    keyboard.push({ key, active: await activeTreeItem() });
  };

  await modelItem.focus();
  keyboard.push({ key: "focus-model", active: await activeTreeItem() });
  await pressAndRecord("Home");
  await pressAndRecord("ArrowRight");
  await pressAndRecord("ArrowDown");
  await pressAndRecord("ArrowRight");
  await pressAndRecord("ArrowLeft");
  await pressAndRecord("ArrowLeft");
  const modelsCollapsedByLeft = await modelsItem.getAttribute("aria-expanded") === "false";
  await pressAndRecord("ArrowRight");
  const modelsExpandedByRight = await modelsItem.getAttribute("aria-expanded") === "true";
  await pressAndRecord("ArrowRight");
  await pressAndRecord("End");
  await pressAndRecord("Home");
  await pressAndRecord("ArrowDown");
  await pressAndRecord("ArrowUp");
  await pressAndRecord("End");
  await pressAndRecord("ArrowUp");
  await pressAndRecord("ArrowUp");
  const focusedModelsBeforeEnter = (await activeTreeItem())?.kind === "models";
  await pressAndRecord("Enter");
  const modelsCollapsedByEnter = await modelsItem.getAttribute("aria-expanded") === "false";
  await pressAndRecord("Enter");
  const modelsExpandedByEnter = await modelsItem.getAttribute("aria-expanded") === "true";

  await modelItem.focus();
  await capture(page, "workspace-explorer", `${sequence}a`, viewport);

  await page.keyboard.press("Shift+F10");
  await contextMenu.waitFor({ state: "visible", timeout: 1_000 });
  const context = {
    menuCount: await contextMenu.count(),
    accessibleName: await contextMenu.getAttribute("aria-label"),
    items: await contextMenu.getByRole("menuitem").allTextContents(),
    enabledItems: await contextMenu.getByRole("menuitem").evaluateAll((nodes) => nodes.filter((node) => !(node instanceof HTMLButtonElement) || !node.disabled).map((node) => node.textContent?.replace(/\s+/g, " ").trim() ?? "")),
    initialFocusInside: await contextMenu.evaluate((node) => node.contains(document.activeElement)),
  };
  await capture(page, "workspace-explorer-context-menu", `${sequence}b`, viewport);
  await page.keyboard.press("Escape");
  await contextMenu.waitFor({ state: "hidden" });
  const contextFocusRestored = await modelItem.evaluate((node) => document.activeElement === node);

  await page.keyboard.press("F2");
  await renameDialog.waitFor({ state: "visible", timeout: 1_000 });
  const renameInput = renameDialog.locator("input");
  const rename = {
    title: (await renameDialog.getByRole("heading").textContent())?.trim() ?? "",
    ariaModal: await renameDialog.getAttribute("aria-modal"),
    accessibleName: await renameDialog.getAttribute("aria-labelledby"),
    initialValue: await renameInput.inputValue(),
    inputFocused: await renameInput.evaluate((node) => document.activeElement === node),
  };
  await capture(page, "workspace-explorer-rename-dialog", `${sequence}c`, viewport, { dialog: "explorer" });
  await renameDialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await renameDialog.waitFor({ state: "hidden" });
  await page.waitForTimeout(30);
  rename.cancelled = true;
  rename.focusRestored = await modelItem.evaluate((node) => document.activeElement === node);

  const finalRovingTabStopCount = await treeItems.evaluateAll((nodes) => nodes.filter((node) => node.getAttribute("tabindex") === "0").length);
  const expectedKeyboardKinds = [
    "model",
    "project",
    "data",
    "models",
    "model",
    "models",
    "models",
    "models",
    "model",
    "reports",
    "project",
    "data",
    "project",
    "reports",
    "model",
    "models",
    "models",
    "models",
  ];
  const observedKeyboardKinds = keyboard.map((entry) => entry.active?.kind ?? null);
  const check = {
    viewport: viewport.id,
    fixture: "existing Corporate Reputation smoke project",
    nativeMutationsInvoked: false,
    initial,
    keyboard,
    observedKeyboardKinds,
    expectedKeyboardKinds,
    modelsCollapsedByLeft,
    modelsExpandedByRight,
    focusedModelsBeforeEnter,
    modelsCollapsedByEnter,
    modelsExpandedByEnter,
    context: { ...context, dismissedWithEscape: true, focusRestored: contextFocusRestored },
    rename,
    finalRovingTabStopCount,
  };
  evidence.checks.workspaceExplorer.push(check);

  const requiredKinds = ["project", "data", "models", "model", "reports"];
  if (initial.treeCount !== 1 || requiredKinds.some((kind) => !initial.kinds.includes(kind))) {
    recordFailure("workspace-explorer-tree-structure", `The open-project workspace did not expose one Project/Data/Models/Reports tree at ${viewport.id}.`, check);
  }
  if (initial.rovingTabStopCount !== 1 || initial.selectedCount !== 1 || finalRovingTabStopCount !== 1) {
    recordFailure("workspace-explorer-roving-tabstop", `The Project Explorer did not retain exactly one roving tree tab stop and one selected item at ${viewport.id}.`, check);
  }
  if (initial.documentHorizontalOverflow || initial.layout.some((entry) => entry?.horizontalOverflow)) {
    recordFailure("workspace-explorer-overflow", `The Project Explorer caused horizontal overflow at ${viewport.id}.`, check);
  }
  if (JSON.stringify(observedKeyboardKinds) !== JSON.stringify(expectedKeyboardKinds)
    || !modelsCollapsedByLeft || !modelsExpandedByRight || !focusedModelsBeforeEnter || !modelsCollapsedByEnter || !modelsExpandedByEnter) {
    recordFailure("workspace-explorer-keyboard-navigation", `The Project Explorer Up/Down/Home/End/Right/Left/Enter contract failed at ${viewport.id}.`, check);
  }
  if (context.menuCount !== 1 || context.accessibleName !== "Project item commands" || !context.initialFocusInside
    || !context.items.some((item) => /Rename/i.test(item)) || !contextFocusRestored) {
    recordFailure("workspace-explorer-keyboard-context-menu", `Shift+F10 did not open, focus, and dismiss the model context menu at ${viewport.id}.`, check);
  }
  if (rename.title !== "Rename Model" || rename.ariaModal !== "true" || !rename.accessibleName || !rename.initialValue
    || !rename.inputFocused || !rename.cancelled || !rename.focusRestored) {
    recordFailure("workspace-explorer-f2-rename-cancel", `F2 rename did not open a focused modal and cancel back to the model node at ${viewport.id}.`, check);
  }
}

async function auditModelContextMenuParity(page, viewport) {
  const target = page.locator(".react-flow__node:has(.smartpls-latent-node)").first();
  const menu = page.locator('.nd-context-menu[role="menu"]');
  const browserDialogs = [];
  const dismissBrowserDialog = async (dialog) => {
    browserDialogs.push(dialog.type());
    await dialog.dismiss();
  };
  page.on("dialog", dismissBrowserDialog);

  const propertiesInitiallyOpen = await page.locator('aside[aria-label="Model properties"]').isVisible().catch(() => false);
  let pointerItems = [];
  let keyboardItems = [];
  let pointerInitialFocus = false;
  let keyboardInitialFocus = false;
  let pointerFocusRestored = false;
  let keyboardFocusRestored = false;
  let enterFocusedProperties = false;

  try {
    await target.click();
    await target.focus();
    await target.click({ button: "right" });
    await menu.waitFor({ state: "visible" });
    await page.waitForFunction(() => document.activeElement?.getAttribute("role") === "menuitem");
    pointerItems = (await menu.getByRole("menuitem").allTextContents()).map((label) => label.trim());
    pointerInitialFocus = await page.evaluate(() => Boolean(document.activeElement?.closest('.nd-context-menu[role="menu"]')));
    await page.keyboard.press("Escape");
    await menu.waitFor({ state: "hidden" });
    pointerFocusRestored = await target.evaluate((node) => document.activeElement === node);

    await target.focus();
    await page.keyboard.press("Shift+F10");
    await menu.waitFor({ state: "visible" });
    await page.waitForFunction(() => document.activeElement?.getAttribute("role") === "menuitem");
    keyboardItems = (await menu.getByRole("menuitem").allTextContents()).map((label) => label.trim());
    keyboardInitialFocus = await page.evaluate(() => Boolean(document.activeElement?.closest('.nd-context-menu[role="menu"]')));
    await page.keyboard.press("Escape");
    await menu.waitFor({ state: "hidden" });
    keyboardFocusRestored = await target.evaluate((node) => document.activeElement === node);

    await target.focus();
    await page.keyboard.press("Enter");
    const editor = page.locator("#nd-model-construct-name");
    await editor.waitFor({ state: "visible" });
    await page.waitForFunction(() => document.activeElement?.id === "nd-model-construct-name", null, { timeout: 2_000 });
    enterFocusedProperties = await editor.evaluate((node) => document.activeElement === node);
  } finally {
    page.off("dialog", dismissBrowserDialog);
    const propertiesNowOpen = await page.locator('aside[aria-label="Model properties"]').isVisible().catch(() => false);
    if (!propertiesInitiallyOpen && propertiesNowOpen) {
      await page.locator('.nd-commandbar button[title="Hide Properties"]').click();
    }
  }

  const sameCommands = JSON.stringify(pointerItems) === JSON.stringify(keyboardItems);
  const includesSelectionEditor = pointerItems.some((label) => label.startsWith("Edit Construct Properties"));
  const check = {
    viewport: viewport.id,
    pointerItems,
    keyboardItems,
    sameCommands,
    includesSelectionEditor,
    pointerInitialFocus,
    keyboardInitialFocus,
    pointerFocusRestored,
    keyboardFocusRestored,
    enterFocusedProperties,
    browserDialogs,
  };
  evidence.checks.contextMenus.push(check);
  if (!sameCommands || !includesSelectionEditor) {
    recordFailure("model-context-command-parity", `Pointer and Shift+F10 did not expose the same selected-construct commands at ${viewport.id}.`, check);
  }
  if (!pointerInitialFocus || !keyboardInitialFocus || !pointerFocusRestored || !keyboardFocusRestored) {
    recordFailure("model-context-menu-focus", `The model context menu did not focus and restore the selected construct at ${viewport.id}.`, check);
  }
  if (!enterFocusedProperties || browserDialogs.length > 0) {
    recordFailure("model-native-properties-edit", `Enter did not focus the native Properties editor without a browser prompt at ${viewport.id}.`, check);
  }
}

async function auditModeratingEffectDialog(page, viewport, sequence) {
  const relationshipEdge = page.locator('.react-flow__edge[data-id="comp-cusa"]');
  const interactionTarget = relationshipEdge.locator(".react-flow__edge-interaction");
  const command = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Moderating Effect/i });
  const dialog = page.locator('.nd-dialog-moderation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    relationshipEdgeCount: await relationshipEdge.count(),
    interactionTargetCount: await interactionTarget.count(),
    selectedEligiblePath: false,
    commandCount: await command.count(),
    commandEnabled: false,
    dialogOpened: false,
    relationshipId: null,
    relationshipLabel: "",
    moderatorId: null,
    moderatorLabel: "",
    method: "",
    automaticMainEffectDisclosure: "",
    disclosureIsExplicit: false,
    createEnabled: false,
    dialogClosed: false,
    focusRestored: false,
  };

  try {
    if (check.relationshipEdgeCount === 1 && check.interactionTargetCount === 1 && check.commandCount === 1) {
      await interactionTarget.dispatchEvent("click");
      await page.waitForFunction(() => document.querySelector('.react-flow__edge[data-id="comp-cusa"]')?.classList.contains("selected"), null, { timeout: 1_000 }).catch(() => null);
      check.selectedEligiblePath = await relationshipEdge.evaluate((node) => node.classList.contains("selected"));
      check.commandEnabled = await command.isEnabled();
      if (check.commandEnabled) {
        await command.click();
        await dialog.waitFor({ state: "visible", timeout: 2_000 });
        check.dialogOpened = true;
        check.relationshipId = await dialog.locator("#nd-moderation-relationship").inputValue();
        check.relationshipLabel = compactCalculationText(await dialog.locator("#nd-moderation-relationship option:checked").textContent());
        check.moderatorId = await dialog.locator("#nd-moderation-moderator").inputValue();
        check.moderatorLabel = compactCalculationText(await dialog.locator("#nd-moderation-moderator option:checked").textContent());
        check.method = compactCalculationText(await dialog.locator(".nd-moderation-summary").textContent());
        check.automaticMainEffectDisclosure = compactCalculationText(await dialog.locator(".nd-dialog-note").textContent());
        check.disclosureIsExplicit = /adds the moderator(?:'|\u2019)s main-effect path to the outcome when it is missing/i.test(check.automaticMainEffectDisclosure);
        check.createEnabled = await dialog.getByRole("button", { name: "Create moderating effect", exact: true }).isEnabled();
        await capture(page, "moderating-effect-dialog", sequence, viewport, { dialog: "moderation" });

        await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
        await dialog.waitFor({ state: "hidden", timeout: 1_000 });
        check.dialogClosed = true;
        await page.waitForTimeout(50);
        check.focusRestored = await command.evaluate((node) => document.activeElement === node);
      }
    }
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click().catch(() => null);
      await dialog.waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
    }
    evidence.checks.moderationAuthoring.push(check);
  }

  if (check.relationshipEdgeCount !== 1 || check.interactionTargetCount !== 1 || !check.selectedEligiblePath) {
    recordFailure("moderation-selected-path-contract", `The browser model could not select the eligible Competence to Customer Satisfaction path at ${viewport.id}.`, check);
  }
  if (check.commandCount !== 1 || !check.commandEnabled || !check.dialogOpened) {
    recordFailure("moderation-command-dialog-contract", `The selected eligible path did not enable and open exactly one Moderating Effect dialog at ${viewport.id}.`, check);
  }
  if (check.relationshipId !== "comp-cusa" || !/Competence/.test(check.relationshipLabel) || !/Customer Satisfaction/.test(check.relationshipLabel)) {
    recordFailure("moderation-selected-relationship-binding", `The Moderating Effect dialog was not bound to the selected Competence-to-Customer Satisfaction relationship at ${viewport.id}.`, check);
  }
  if (!check.moderatorId || !check.moderatorLabel || !/Two-stage product score/i.test(check.method) || !check.createEnabled) {
    recordFailure("moderation-dialog-setup-contract", `The Moderating Effect dialog did not expose a valid moderator, two-stage method, and enabled create command at ${viewport.id}.`, check);
  }
  if (!check.disclosureIsExplicit) {
    recordFailure("moderation-automatic-main-effect-disclosure", `The Moderating Effect dialog did not explicitly disclose automatic moderator-to-outcome main-effect creation at ${viewport.id}.`, check);
  }
  if (!check.dialogClosed || !check.focusRestored) {
    recordFailure("moderation-dialog-close-focus", `Cancel did not close the Moderating Effect dialog and restore toolbar focus at ${viewport.id}.`, check);
  }
}

async function auditHigherOrderConstructDialog(page, viewport, sequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    constructCount: 0,
    constructLabels: [],
    commandCount: 0,
    commandEnabled: false,
    dialogOpened: false,
    componentCount: 0,
    selectedComponents: [],
    method: "",
    stage1: "",
    stage2: "",
    scopeDisclosure: "",
    exactBoundedScope: false,
    unsupportedInferenceControls: 0,
    createEnabled: false,
    dialogOverflow: false,
    pageOverflow: false,
    dialogClosed: false,
    focusRestored: false,
  };

  const command = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Higher-Order Construct/i });
  const dialog = page.locator('.nd-dialog-higher-order[role="dialog"]');
  const propertiesToggle = page.locator('.nd-commandbar[role="toolbar"] button.nd-pane-toggle');

  const setPropertiesOpen = async (open) => {
    const pressed = await propertiesToggle.getAttribute("aria-pressed");
    if ((pressed === "true") !== open) await propertiesToggle.click();
    const properties = page.locator("aside.nd-model-inspector");
    if (open) await properties.waitFor({ state: "visible", timeout: 2_000 });
    else await properties.waitFor({ state: "hidden", timeout: 2_000 }).catch(() => null);
  };

  const clearSelection = async () => {
    await setPropertiesOpen(false);
    const pane = page.locator(".react-flow__pane");
    await pane.waitFor({ state: "visible", timeout: 2_000 });
    const box = await pane.boundingBox();
    if (!box) throw new Error("The HOC browser fixture did not expose model-canvas bounds.");
    await pane.click({ position: { x: Math.max(8, box.width - 24), y: 24 } });
    await page.locator(".react-flow__node-latent.selected").waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
  };

  const createAndRename = async (indicator, name, shortName, index) => {
    const variable = page.locator("button.nd-variable-item").filter({ hasText: new RegExp(`^${indicator}$`) });
    await variable.waitFor({ state: "visible", timeout: 2_000 });
    await variable.click();
    const node = page.locator(".react-flow__node-latent").nth(index);
    await node.waitFor({ state: "visible", timeout: 2_000 });
    await setPropertiesOpen(true);
    const properties = page.locator("aside.nd-model-inspector");
    const nameInput = properties.getByLabel("Name", { exact: true });
    const shortNameInput = properties.getByLabel("Short name", { exact: true });
    await nameInput.fill(name);
    await nameInput.press("Enter");
    await shortNameInput.fill(shortName);
    await shortNameInput.press("Enter");
    await node.filter({ hasText: name }).waitFor({ state: "visible", timeout: 2_000 });
    await clearSelection();
  };

  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadHocFixture === "function");
    if (!check.fixtureApiPresent) throw new Error("loadHocFixture is not exposed by the production smoke API.");
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadHocFixture());
    await setSurface(page, "model");

    await createAndRename("capability", "Capability", "CAP", 0);
    await createAndRename("resources", "Resources", "RES", 1);
    await createAndRename("performance", "Performance", "PERF", 2);

    const nodes = page.locator(".react-flow__node-latent");
    check.constructCount = await nodes.count();
    check.constructLabels = await nodes.allTextContents();
    check.commandCount = await command.count();
    check.commandEnabled = check.commandCount === 1 && await command.isEnabled();
    if (check.commandEnabled) {
      await command.click();
      await dialog.waitFor({ state: "visible", timeout: 2_000 });
      check.dialogOpened = true;
      const componentCheckboxes = dialog.locator('input[type="checkbox"]');
      const capability = dialog.getByRole("checkbox", { name: /Capability/ });
      const resources = dialog.getByRole("checkbox", { name: /Resources/ });
      await capability.waitFor({ state: "visible", timeout: 2_000 });
      check.componentCount = await componentCheckboxes.count();
      await capability.check();
      await resources.check();
      check.selectedComponents = await componentCheckboxes.evaluateAll((elements) => elements
        .filter((element) => element.checked)
        .map((element) => element.closest("label")?.innerText?.replace(/\s+/g, " ").trim() ?? ""));
      const summary = await dialog.locator(".nd-hoc-summary").evaluate((element) => Object.fromEntries(Array.from(element.querySelectorAll(":scope > div")).map((row) => [
        row.querySelector("dt")?.textContent?.trim() ?? "",
        row.querySelector("dd")?.textContent?.trim() ?? "",
      ])));
      check.method = summary.Method ?? "";
      check.stage1 = summary["Stage 1"] ?? "";
      check.stage2 = summary["Stage 2"] ?? "";
      check.scopeDisclosure = compactCalculationText(await dialog.locator(".nd-dialog-note").textContent());
      check.exactBoundedScope = /one HOC-to-outcome relationship/i.test(check.scopeDisclosure)
        && /no other structural path/i.test(check.scopeDisclosure)
        && /PLS-SEM Algorithm/i.test(check.scopeDisclosure)
        && /path weighting/i.test(check.scopeDisclosure)
        && /standardized data/i.test(check.scopeDisclosure)
        && /listwise deletion/i.test(check.scopeDisclosure)
        && /bootstrapping and permutation inference remain unavailable/i.test(check.scopeDisclosure);
      check.unsupportedInferenceControls = await dialog.locator('[id*="bootstrap"], [id*="permutation"], [id*="confidence"], input[type="number"]').count();
      check.createEnabled = await dialog.getByRole("button", { name: "Create higher-order construct", exact: true }).isEnabled();
      const overflow = await dialog.evaluate((element) => ({
        dialog: element.scrollWidth > element.clientWidth + 2,
        page: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) > document.documentElement.clientWidth + 2,
      }));
      check.dialogOverflow = overflow.dialog;
      check.pageOverflow = overflow.page;
      await capture(page, "higher-order-dialog", sequence, viewport, { dialog: "higher-order" });
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
      await dialog.waitFor({ state: "hidden", timeout: 1_000 });
      check.dialogClosed = true;
      await page.waitForTimeout(50);
      check.focusRestored = await command.evaluate((element) => document.activeElement === element);
    }
  } catch (error) {
    check.error = error instanceof Error ? error.message : String(error);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click().catch(() => null);
      await dialog.waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
    }
    evidence.checks.higherOrderAuthoring.push(check);
  }

  if (!check.fixtureApiPresent || check.fixture?.variables !== 3 || check.fixture?.models !== 0) {
    recordFailure("hoc-data-only-fixture", `The browser HOC audit did not begin from the expected three-variable, zero-model fixture at ${viewport.id}.`, check);
  }
  if (check.constructCount !== 3 || !["Capability", "Resources", "Performance"].every((label) => check.constructLabels.some((value) => value.includes(label)))) {
    recordFailure("hoc-visible-model-authoring", `The HOC browser audit did not visibly create the three ordinary measured constructs at ${viewport.id}.`, check);
  }
  if (check.commandCount !== 1 || !check.commandEnabled || !check.dialogOpened || check.componentCount !== 3) {
    recordFailure("hoc-command-dialog-contract", `The model toolbar did not expose and open one usable Higher-Order Construct dialog at ${viewport.id}.`, check);
  }
  if (check.selectedComponents.length !== 2 || !check.selectedComponents.some((value) => value.includes("Capability")) || !check.selectedComponents.some((value) => value.includes("Resources"))) {
    recordFailure("hoc-component-selection", `The HOC dialog did not retain the explicit Capability and Resources component selection at ${viewport.id}.`, check);
  }
  if (!/Reflective.+reflective disjoint two-stage/i.test(check.method) || !/component scores/i.test(check.stage1) || !/generated HOC indicators/i.test(check.stage2)) {
    recordFailure("hoc-two-stage-disclosure", `The HOC dialog did not explain the exact reflective-reflective disjoint two-stage method at ${viewport.id}.`, check);
  }
  if (!check.exactBoundedScope || check.unsupportedInferenceControls !== 0 || !check.createEnabled) {
    recordFailure("hoc-bounded-scope", `The HOC dialog did not truthfully expose the bounded point-estimate-only calculation scope at ${viewport.id}.`, check);
  }
  if (check.dialogOverflow || check.pageOverflow) {
    recordFailure("hoc-dialog-overflow", `The HOC dialog overflowed horizontally at ${viewport.id}.`, check);
  }
  if (!check.dialogClosed || !check.focusRestored) {
    recordFailure("hoc-dialog-close-focus", `Cancel did not close the HOC dialog and restore toolbar focus at ${viewport.id}.`, check);
  }
}

function compactCalculationText(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function normalizeCalculationPlan(value) {
  return compactCalculationText(value).replace(/\u00d7/g, "x");
}

function normalizeGroupDifference(value) {
  return compactCalculationText(value)
    .replace(/\s*(?:\u2212|\u2013|\u2014)\s*/g, " - ")
    .replace(/Group A\s*-\s*Group B/gi, "Group A - Group B")
    .replace(/\s+/g, " ")
    .trim();
}

function isGenericCalculateLabel(value) {
  return /^Calculate(?:\u2026|\.\.\.)?$/i.test(compactCalculationText(value));
}

function calculationOption(dialog, kind) {
  return dialog.locator("#nd-calculation-method-" + kind);
}

async function ensureCalculateTriggerEnabled(trigger) {
  if (trigger && await trigger.count() === 1 && await trigger.isDisabled()) {
    await trigger.evaluate((node) => { node.disabled = false; });
  }
}

async function auditCalculateCommandSurface(page, viewport) {
  const calculationLike = /calculate|algorithm|bootstrap|permutation|randomization|construct prediction|consistent pls|weighted pls|importance-performance|necessary condition/i;
  const toolbarLabels = (await page.locator('.nd-commandbar[role="toolbar"] button').allTextContents())
    .map(compactCalculationText);
  const toolbarCalculationCommands = toolbarLabels.filter((label) => calculationLike.test(label));
  const calculateMenuTrigger = page.locator('.nd-menubar [role="menuitem"][aria-haspopup="menu"]')
    .filter({ hasText: /^Calculate$/ });
  const calculateMenuTriggerCount = await calculateMenuTrigger.count();
  let menuLabels = [];
  let menuCalculationCommands = [];
  if (calculateMenuTriggerCount === 1) {
    await calculateMenuTrigger.click();
    const popupId = await calculateMenuTrigger.getAttribute("aria-controls");
    const popup = popupId ? page.locator("#" + popupId) : page.locator('.nd-menu-popup[role="menu"]');
    await popup.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    menuLabels = (await popup.locator('[role="menuitem"] > span').allTextContents()).map(compactCalculationText);
    menuCalculationCommands = menuLabels.filter((label) => calculationLike.test(label));
    await page.keyboard.press("Escape");
    await popup.waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
  }
  const toolbarContract = toolbarCalculationCommands.length === 1
    && isGenericCalculateLabel(toolbarCalculationCommands[0]);
  const menuContract = calculateMenuTriggerCount === 1
    && menuCalculationCommands.length === 1
    && isGenericCalculateLabel(menuCalculationCommands[0]);
  return {
    viewport: viewport.id,
    toolbarLabels,
    toolbarCalculationCommands,
    toolbarContract,
    calculateMenuTriggerCount,
    menuLabels,
    menuCalculationCommands,
    menuContract,
    soleGenericCalculateCommand: toolbarContract && menuContract,
  };
}

async function openCalculationDialogForInspection(page, viewport) {
  const commandSurface = await auditCalculateCommandSurface(page, viewport);
  const trigger = page.locator('.nd-commandbar[role="toolbar"] button')
    .filter({ hasText: /^Calculate(?:\u2026|\.\.\.)?$/i })
    .first();
  if (await trigger.count() !== 1) {
    recordFailure("calculation-dialog-trigger-missing", "No generic Calculate command was rendered in the model toolbar at " + viewport.id + ".", commandSurface);
    return { opened: false, trigger: null, originallyDisabled: false, restore: async () => undefined, commandSurface };
  }
  const originallyDisabled = await trigger.isDisabled();
  if (originallyDisabled) {
    await ensureCalculateTriggerEnabled(trigger);
    recordSkip("calculation-runtime-command", "The production web preview truthfully disables calculation because native IPC is unavailable. The harness temporarily enables only the existing dialog trigger to inspect configuration UI; it never starts a job.", {
      viewport: viewport.id,
      screenshotStillCaptured: true,
      runStateFabricated: false,
    });
  }
  await trigger.focus();
  await trigger.click();
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
  const opened = await dialog.isVisible().catch(() => false);
  if (!opened) {
    recordFailure("calculation-dialog-open", "The generic Calculate command did not open its dialog at " + viewport.id + ".", commandSurface);
  }
  return {
    opened,
    trigger,
    originallyDisabled,
    commandSurface,
    restore: async () => {
      if (originallyDisabled && await trigger.count()) {
        await trigger.evaluate((node) => { node.disabled = true; });
      }
    },
  };
}

async function reopenCalculationDialog(page, viewport, trigger, failureId) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  if (await dialog.isVisible().catch(() => false)) return true;
  if (!trigger || await trigger.count() !== 1) {
    recordFailure(failureId, "No generic Calculate command was available at " + viewport.id + ".");
    return false;
  }
  await ensureCalculateTriggerEnabled(trigger);
  await trigger.focus();
  await trigger.click({ timeout: 2_000 });
  await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
  const opened = await dialog.isVisible().catch(() => false);
  if (!opened) {
    recordFailure(failureId, "The generic Calculate command did not open its dialog at " + viewport.id + ".");
  }
  return opened;
}

async function inspectSelectedMethodLinkage(dialog, expectedKind) {
  const expected = nativeCalculationMethods.find((method) => method.kind === expectedKind);
  const selected = dialog.locator('#nd-calculation-method-list [role="option"][aria-selected="true"]');
  const selectedCount = await selected.count();
  const selectedId = selectedCount === 1 ? await selected.getAttribute("id") : null;
  const selectedLabel = selectedCount === 1
    ? compactCalculationText(await selected.locator("strong").textContent().catch(() => ""))
    : "";
  const panel = dialog.locator('#nd-calculation-panel[role="region"]');
  const panelCount = await panel.count();
  const panelId = panelCount === 1 ? await panel.getAttribute("id") : null;
  const panelLabelledBy = panelCount === 1 ? await panel.getAttribute("aria-labelledby") : null;
  const heading = panel.locator("h3");
  const headingCount = await heading.count();
  const headingId = headingCount === 1 ? await heading.getAttribute("id") : null;
  const headingLabel = headingCount === 1
    ? compactCalculationText(await heading.textContent().catch(() => ""))
    : "";
  const expectedHeadingId = "nd-calculation-panel-" + expectedKind + "-title";
  const linkage = selectedCount === 1
    && selectedId === "nd-calculation-method-" + expectedKind
    && selectedLabel === expected?.label
    && panelCount === 1
    && panelId === "nd-calculation-panel"
    && panelLabelledBy === expectedHeadingId
    && headingCount === 1
    && headingId === expectedHeadingId
    && headingLabel === expected?.label;
  return {
    expectedKind,
    expectedLabel: expected?.label ?? null,
    selectedCount,
    selectedId,
    selectedLabel,
    panelCount,
    panelId,
    panelLabelledBy,
    headingCount,
    headingId,
    headingLabel,
    linkage,
  };
}

async function inspectCalculationTruthAndOverflow(dialog) {
  const fabricatedRunState = await dialog.evaluate((node) => {
    const text = node.textContent ?? "";
    return {
      statusSections: node.querySelectorAll('.nd-run-progress, [aria-label="Calculation status"]').length,
      progressElements: node.querySelectorAll("progress").length,
      calculationLogs: Array.from(node.querySelectorAll("summary"))
        .filter((summary) => /calculation log/i.test(summary.textContent ?? "")).length,
      iterationRows: (text.match(/Iteration\s+\d+/gi) ?? []).length,
    };
  });
  const horizontalOverflow = await dialog.evaluate((node) => {
    const rect = node.getBoundingClientRect();
    return {
      dialogScrollWidth: node.scrollWidth,
      dialogClientWidth: node.clientWidth,
      dialogOutsideViewport: rect.left < -2 || rect.right > window.innerWidth + 2,
      dialogContentOverflow: node.scrollWidth > node.clientWidth + 2,
      pageOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth)
        > document.documentElement.clientWidth + 2,
    };
  });
  return {
    fabricatedRunState,
    noFabricatedRunState: Object.values(fabricatedRunState).every((count) => count === 0),
    horizontalOverflow,
    noHorizontalOverflow: !horizontalOverflow.dialogOutsideViewport
      && !horizontalOverflow.dialogContentOverflow
      && !horizontalOverflow.pageOverflow,
  };
}

async function closeCalculationAndCheckFocus(page, dialog, trigger) {
  await ensureCalculateTriggerEnabled(trigger);
  const closeButton = dialog.locator("footer").getByRole("button", { name: "Close", exact: true });
  const closeButtonCount = await closeButton.count();
  if (closeButtonCount === 1) await closeButton.click({ timeout: 2_000 });
  await dialog.waitFor({ state: "hidden", timeout: 1_000 }).catch(() => null);
  const dialogClosed = await dialog.isHidden().catch(() => true);
  let focusRestored = false;
  if (trigger && await trigger.count() === 1) {
    const handle = await trigger.elementHandle();
    if (handle) {
      await page.waitForFunction((node) => document.activeElement === node, handle, { timeout: 1_000 }).catch(() => null);
      focusRestored = await trigger.evaluate((node) => document.activeElement === node).catch(() => false);
    }
  }
  return { closeButtonCount, dialogClosed, focusRestored };
}

async function selectCalculationMethod(dialog, kind) {
  const option = calculationOption(dialog, kind);
  await option.waitFor({ state: "visible", timeout: 2_000 });
  await option.click({ timeout: 2_000 });
  await dialog.locator("#nd-calculation-method-" + kind + '[aria-selected="true"]')
    .waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
  return {
    pointerSelected: await option.getAttribute("aria-selected") === "true",
    linkage: await inspectSelectedMethodLinkage(dialog, kind),
  };
}

async function auditCalculationCatalogDialog(page, viewport, sequence, trigger, commandSurface) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const expectedLabels = nativeCalculationMethods.map((method) => method.label);
  const keyboardExpectations = [
    { key: "Home", focusedKind: "pls_algorithm", selectedKind: "wpls" },
    { key: "ArrowDown", focusedKind: "plsc", selectedKind: "wpls" },
    { key: "Enter", focusedKind: "plsc", selectedKind: "plsc" },
    { key: "End", focusedKind: "regression", selectedKind: "plsc" },
    { key: "ArrowUp", focusedKind: "pca", selectedKind: "plsc" },
    { key: "ArrowDown", focusedKind: "regression", selectedKind: "plsc" },
  ];
  const check = {
    viewport: viewport.id,
    commandSurface,
    searchInitiallyFocused: false,
    listboxCount: 0,
    optionCount: 0,
    optionLabels: [],
    expectedLabels,
    countStatus: "",
    searchAliases: [],
    pointerSelected: false,
    keyboardSteps: [],
    keyboardContract: false,
    linkage: null,
    truthAndOverflow: null,
    draftCancellation: null,
    closeFocus: null,
  };
  let expectedSearchAliasCount = 0;

  try {
    const search = dialog.getByLabel("Find a method", { exact: true });
    await search.waitFor({ state: "visible", timeout: 1_000 });
    await page.waitForFunction(() => document.activeElement?.id === "nd-calculation-method-search", undefined, { timeout: 1_000 }).catch(() => null);
    check.searchInitiallyFocused = await search.evaluate((node) => document.activeElement === node);
    check.listboxCount = await listbox.count();
    const options = listbox.locator('[role="option"]');
    check.optionCount = await options.count();
    check.optionLabels = (await options.locator("strong").allTextContents()).map(compactCalculationText);
    check.countStatus = compactCalculationText(await dialog.locator('.nd-method-count[role="status"]').textContent().catch(() => ""));

    const searchableAliases = [
      { query: "survey weights", expectedKind: "wpls", expectedLabel: "Weighted PLS" },
      { query: "generalized structured component", expectedKind: "gsca", expectedLabel: "GSCA" },
      { query: "composite residual", expectedKind: "cca", expectedLabel: "CCA composite residual diagnostics" },
      { query: "confirmatory tetrad", expectedKind: "cta_pls", expectedLabel: "Confirmatory Tetrad Analysis" },
      { query: "importance performance", expectedKind: "ipma", expectedLabel: "Importance-Performance Map Analysis" },
      { query: "confirmatory factor maximum likelihood", expectedKind: "cbsem", expectedLabel: "CB-SEM / CFA" },
      { query: "randomization", expectedKind: "pls_permutation", expectedLabel: "Structural Path Randomization" },
      { query: "prospective power", expectedKind: "pls_sample_size_power", expectedLabel: "PLS-SEM Sample Size and Power" },
      { query: "measurement invariance", expectedKind: "mga", expectedLabel: "MICOM and Two-Group Permutation MGA" },
      { query: "ce-fdh bottleneck", expectedKind: "nca", expectedLabel: "Necessary Condition Analysis" },
      { query: "principal component eigenvalue", expectedKind: "pca", expectedLabel: "Principal Component Analysis" },
      { query: "ordinary least squares hc3", expectedKind: "regression", expectedLabel: "Regression" },
    ].filter((alias) => nativeCalculationMethods.some((method) => method.kind === alias.expectedKind));
    expectedSearchAliasCount = searchableAliases.length;
    for (const alias of searchableAliases) {
      await search.fill(alias.query);
      await page.waitForFunction((expectedId) => {
        const optionsNow = Array.from(document.querySelectorAll('#nd-calculation-method-list [role="option"]'));
        return optionsNow.length === 1 && optionsNow[0].id === expectedId;
      }, "nd-calculation-method-" + alias.expectedKind, { timeout: 1_000 }).catch(() => null);
      const filtered = listbox.locator('[role="option"]');
      const filteredCount = await filtered.count();
      const filteredOption = filteredCount > 0 ? filtered.first() : null;
      check.searchAliases.push({
        query: alias.query,
        expectedKind: alias.expectedKind,
        expectedLabel: alias.expectedLabel,
        optionCount: filteredCount,
        optionId: filteredOption ? await filteredOption.getAttribute("id", { timeout: 1_000 }).catch(() => null) : null,
        optionLabel: filteredOption
          ? compactCalculationText(await filteredOption.locator("strong").textContent({ timeout: 1_000 }).catch(() => ""))
          : "",
        status: compactCalculationText(await dialog.locator('.nd-method-count[role="status"]').textContent().catch(() => "")),
      });
    }
    await search.fill("");
    await page.waitForFunction((expectedCount) => (
      document.querySelectorAll('#nd-calculation-method-list [role="option"]').length === expectedCount
    ), nativeCalculationMethods.length, { timeout: 5_000 }).catch(async (error) => {
      const observed = await page.evaluate(() => ({
        query: document.querySelector('#nd-calculation-method-search')?.value ?? null,
        optionIds: Array.from(document.querySelectorAll('#nd-calculation-method-list [role="option"]'))
          .map((option) => option.id),
        countStatus: document.querySelector('.nd-method-count[role="status"]')?.textContent ?? null,
      }));
      throw new Error(`Calculation catalogue did not repopulate to ${nativeCalculationMethods.length} methods after clearing search: ${JSON.stringify(observed)}. ${error.message}`);
    });

    const weighted = await selectCalculationMethod(dialog, "wpls");
    check.pointerSelected = weighted.pointerSelected;
    const weightedOption = calculationOption(dialog, "wpls");
    await weightedOption.focus();
    for (const expected of keyboardExpectations) {
      await page.keyboard.press(expected.key);
      const expectedFocusedId = "nd-calculation-method-" + expected.focusedKind;
      const expectedSelectedId = "nd-calculation-method-" + expected.selectedKind;
      await page.waitForFunction(({ focusedId, selectedId }) => (
        document.activeElement?.id === focusedId
        && document.querySelector('#nd-calculation-method-list [role="option"][aria-selected="true"]')?.id === selectedId
      ), { focusedId: expectedFocusedId, selectedId: expectedSelectedId }, { timeout: 1_000 }).catch(() => null);
      check.keyboardSteps.push(await page.evaluate(({ key, focusedId, selectedId }) => {
        const optionNodes = Array.from(document.querySelectorAll('#nd-calculation-method-list [role="option"]'));
        return {
          key,
          expectedFocusedId: focusedId,
          expectedSelectedId: selectedId,
          focusedId: document.activeElement?.id ?? null,
          selectedId: document.querySelector('#nd-calculation-method-list [role="option"][aria-selected="true"]')?.id ?? null,
          tabStopIds: optionNodes.filter((option) => option.tabIndex === 0).map((option) => option.id),
        };
      }, { key: expected.key, focusedId: expectedFocusedId, selectedId: expectedSelectedId }));
    }
    check.keyboardContract = check.keyboardSteps.length === keyboardExpectations.length
      && check.keyboardSteps.every((step) => (
        step.focusedId === step.expectedFocusedId
        && step.selectedId === step.expectedSelectedId
        && step.tabStopIds.length === 1
        && step.tabStopIds[0] === step.expectedFocusedId
      ));

    await selectCalculationMethod(dialog, "pls_algorithm");
    check.linkage = await inspectSelectedMethodLinkage(dialog, "pls_algorithm");
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "calculation-dialog", sequence, viewport, { dialog: "calculation" });
    await attemptTruthfulRunningCapture(page, viewport);

    const iterations = dialog.getByLabel("Maximum iterations", { exact: true });
    const originalIterations = await iterations.inputValue();
    const originalNumber = Number(originalIterations);
    const draftIterations = String(originalNumber >= 100_000 ? originalNumber - 100 : originalNumber + 100);
    await iterations.fill(draftIterations);
    const editedIterations = await iterations.inputValue();
    const firstClose = await closeCalculationAndCheckFocus(page, dialog, trigger);

    const reopened = await reopenCalculationDialog(page, viewport, trigger, "calculation-draft-reopen");
    let reopenedIterations = null;
    let reopenedSelectedId = null;
    if (reopened) {
      reopenedIterations = await dialog.getByLabel("Maximum iterations", { exact: true }).inputValue().catch(() => null);
      reopenedSelectedId = await dialog.locator('#nd-calculation-method-list [role="option"][aria-selected="true"]')
        .getAttribute("id").catch(() => null);
      check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
    }
    check.draftCancellation = {
      originalIterations,
      draftIterations,
      editedIterations,
      firstClose,
      reopened,
      reopenedIterations,
      reopenedSelectedId,
      cancelled: editedIterations === draftIterations
        && reopened
        && reopenedIterations === originalIterations
        && reopenedSelectedId === "nd-calculation-method-pls_algorithm",
    };
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
  }

  evidence.checks.calculationCatalog.push(check);
  const aliasesCorrect = check.searchAliases.length === expectedSearchAliasCount
    && check.searchAliases.every((entry) => (
      entry.optionCount === 1
      && entry.optionId === "nd-calculation-method-" + entry.expectedKind
      && entry.optionLabel === entry.expectedLabel
      && entry.status === "1 method"
    ));
  if (!commandSurface.soleGenericCalculateCommand) {
    recordFailure("calculation-command-surface", "Calculate was not the sole generic calculation command in both the menu and toolbar at " + viewport.id + ".", check);
  }
  if (!check.searchInitiallyFocused || check.listboxCount !== 1 || check.optionCount !== expectedLabels.length
    || JSON.stringify(check.optionLabels) !== JSON.stringify(expectedLabels)
    || check.countStatus !== `${nativeCalculationMethods.length} methods`) {
    recordFailure("calculation-method-listbox", `The calculation dialog did not expose the exact ordered ${nativeCalculationMethods.length}-option searchable listbox at ${viewport.id}.`, check);
  }
  if (!aliasesCorrect) {
    recordFailure("calculation-method-search-aliases", `One or more searchable aliases did not isolate their visible calculation method at ${viewport.id}.`, check);
  }
  if (!check.pointerSelected || !check.keyboardContract) {
    recordFailure("calculation-listbox-interaction", "Pointer selection or Up/Down/Home/End/Enter roving behavior failed at " + viewport.id + ".", check);
  }
  if (!check.linkage?.linkage) {
    recordFailure("calculation-selected-settings-linkage", "The selected method and labelled settings region were not linked at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState) {
    recordFailure("calculation-fabricated-run-state", "The idle calculation dialog exposed fabricated progress, logs, or iteration rows at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("calculation-dialog-horizontal-overflow", "The calculation dialog overflowed horizontally at " + viewport.id + ".", check);
  }
  if (!check.draftCancellation?.cancelled) {
    recordFailure("calculation-local-draft-cancellation", "Closing the dialog did not discard its local Maximum iterations draft at " + viewport.id + ".", check);
  }
  if (!check.draftCancellation?.firstClose?.focusRestored || !check.closeFocus?.focusRestored) {
    recordFailure("calculation-dialog-close-focus", "Close did not restore focus to the generic Calculate command at " + viewport.id + ".", check);
  }
}

async function auditPlscDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    scopeLabel: "",
    scopeDetail: "",
    pcaWeightingOptionCount: 0,
    pcaWeightingDisabled: false,
    startCommandCount: 0,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "plsc-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "plsc");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;
    const scope = dialog.locator(".nd-setting-note").filter({ hasText: "Supported setup" });
    check.scopeLabel = compactCalculationText(await scope.locator("span").textContent().catch(() => ""));
    check.scopeDetail = compactCalculationText(await scope.locator("strong").textContent().catch(() => ""));
    const pcaWeighting = dialog.locator('#nd-calculation-weighting option[value="pca"]');
    check.pcaWeightingOptionCount = await pcaWeighting.count();
    check.pcaWeightingDisabled = check.pcaWeightingOptionCount === 1
      && await pcaWeighting.evaluate((option) => option.disabled).catch(() => false);
    check.startCommandCount = await dialog.getByRole("button", { name: "Start consistent PLS", exact: true }).count();
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "plsc-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.plsc.push(check);
  }
  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("plsc-method-editor-linkage", "Consistent PLS did not open as the selected, labelled method editor at " + viewport.id + ".", check);
  }
  if (check.scopeLabel !== "Supported setup"
    || check.scopeDetail !== "Reflective constructs with at least two indicators each; path or factor weighting; raw observations with listwise deletion"
    || check.pcaWeightingOptionCount !== 1 || !check.pcaWeightingDisabled) {
    recordFailure("plsc-validated-scope", "Consistent PLS did not disclose its validated scope and disabled PCA weighting at " + viewport.id + ".", check);
  }
  if (check.startCommandCount !== 1) {
    recordFailure("plsc-start-command", "Consistent PLS did not expose exactly one Start consistent PLS command at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("plsc-dialog-truth-layout", "The idle Consistent PLS editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("plsc-dialog-close-focus", "Close did not dismiss Consistent PLS and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditWplsDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    resultData: "",
    caseWeightCount: 0,
    caseWeightPlaceholder: "",
    caseWeightNote: "",
    blockerText: "",
    missingWeightBlocker: false,
    startCommandCount: 0,
    startCommandDisabled: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "wpls-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "wpls");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;
    const resultData = dialog.locator(".nd-setting-note").filter({ hasText: "Result data" });
    check.resultData = compactCalculationText(await resultData.locator("strong").textContent().catch(() => ""));
    const caseWeight = dialog.locator("#nd-calculation-case-weight");
    check.caseWeightCount = await caseWeight.count();
    if (check.caseWeightCount === 1) {
      await caseWeight.selectOption("");
      check.caseWeightPlaceholder = compactCalculationText(await caseWeight.locator('option[value=""]').textContent().catch(() => ""));
    }
    const caseWeightNote = dialog.locator(".nd-setting-note").filter({ hasText: "Case weights" });
    check.caseWeightNote = compactCalculationText(await caseWeightNote.locator("strong").textContent().catch(() => ""));
    const blocker = dialog.locator('.nd-blocker[role="alert"]');
    await blocker.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.blockerText = compactCalculationText(await blocker.textContent().catch(() => ""));
    check.missingWeightBlocker = check.blockerText.includes("Choose a positive numeric case-weight variable");
    const start = dialog.getByRole("button", { name: "Start weighted PLS", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "wpls-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.wpls.push(check);
  }
  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("wpls-method-editor-linkage", "Weighted PLS did not open as the selected, labelled method editor at " + viewport.id + ".", check);
  }
  if (check.resultData !== "Standardized (fixed)" || check.caseWeightCount !== 1
    || check.caseWeightPlaceholder !== "Select a numeric variable"
    || check.caseWeightNote !== "Positive finite values; the complete column is checked before calculation") {
    recordFailure("wpls-editor-contract", "Weighted PLS did not expose its fixed result data and case-weight editor contract at " + viewport.id + ".", check);
  }
  if (!check.missingWeightBlocker || check.startCommandCount !== 1 || !check.startCommandDisabled) {
    recordFailure("wpls-missing-weight-blocker", "Weighted PLS did not show its missing-weight blocker and disabled start command at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("wpls-dialog-truth-layout", "The idle Weighted PLS editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("wpls-dialog-close-focus", "Close did not dismiss Weighted PLS and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditGscaDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    weighting: "",
    resultData: "",
    estimator: "",
    scope: "",
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    readinessText: "",
    previewRuntimeBlockerVisible: false,
    modelScopeBlockerAbsent: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "gsca-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "gsca");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;

    check.weighting = compactCalculationText(await dialog.locator(".nd-setting-note").filter({ hasText: "Weighting scheme" }).locator("strong").textContent().catch(() => ""));
    check.resultData = compactCalculationText(await dialog.locator(".nd-setting-note").filter({ hasText: "Result data" }).locator("strong").textContent().catch(() => ""));
    check.estimator = compactCalculationText(await dialog.locator("#nd-calculation-gsca-estimator strong").textContent().catch(() => ""));
    check.scope = compactCalculationText(await dialog.locator("#nd-calculation-gsca-scope strong").textContent().catch(() => ""));
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start GSCA", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.readinessText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.previewRuntimeBlockerVisible = check.readinessText.includes("Calculations require the offline QuickPLS desktop runtime");
    check.modelScopeBlockerAbsent = !/requires at least two component constructs|requires at least one recursive structural path|must participate in the structural model|does not support control paths|does not support covariance paths|does not support interaction or higher-order constructs/i.test(check.readinessText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "gsca-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.gsca.push(check);
    recordSkip("gsca-completed-results-browser", "Completed GSCA results are not synthesized in the browser visual harness. A genuine packaged-Tauri ALS calculation, export, save, and reopen remain required.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the deterministic mixed reflective/formative GSCA fixture through the packaged Tauri application and capture authoritative lifecycle and result evidence.",
    });
  }

  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("gsca-method-editor-linkage", "GSCA did not open as the selected, labelled method editor at " + viewport.id + ".", check);
  }
  if (check.weighting !== "Path weighting (fixed)"
    || check.resultData !== "Standardized (fixed)"
    || check.estimator !== "Joint global least-squares alternating least squares; fixed +1 initialization"
    || !check.scope.includes("1e-7 objective-and-weight stop criterion")
    || !check.scope.includes("No controls, covariance paths, interactions, higher-order constructs, case weights, multigroup analysis, GSCA bootstrapping, or other inference")) {
    recordFailure("gsca-bounded-scope", "The GSCA editor did not disclose its exact joint-ALS, standardized, listwise, point-estimate-only scope at " + viewport.id + ".", check);
  }
  if (check.unsupportedControlCount !== 0) {
    recordFailure("gsca-settings-contract", "The GSCA editor exposed unsupported PLS, resampling, seed, worker, or case-weight controls at " + viewport.id + ".", check);
  }
  if (check.startCommandCount !== 1 || !check.startCommandDisabled
    || !check.previewRuntimeBlockerVisible || !check.modelScopeBlockerAbsent) {
    recordFailure("gsca-command-and-readiness-boundary", "The GSCA editor did not expose one truthful command with the valid sample model blocked only by the browser runtime at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("gsca-dialog-truth-layout", "The idle GSCA editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("gsca-dialog-close-focus", "Close did not dismiss GSCA and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditCcaDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    resultDataLabel: "",
    resultDataDetail: "",
    scopeLabel: "",
    scopeDetail: "",
    missingDataLabel: "",
    missingDataDetail: "",
    pcaWeightingOptionCount: 0,
    pcaWeightingDisabled: false,
    maximumIterationsCount: 0,
    toleranceCount: 0,
    resamplingControlCount: 0,
    caseWeightControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    readinessText: "",
    previewRuntimeBlockerVisible: false,
    ccaModelScopeBlockerAbsent: false,
    descriptiveBoundaryVisible: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "cca-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "cca");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;

    const resultData = dialog.locator(".nd-setting-note").filter({ hasText: "Result data" });
    check.resultDataLabel = compactCalculationText(await resultData.locator("span").textContent().catch(() => ""));
    check.resultDataDetail = compactCalculationText(await resultData.locator("strong").textContent().catch(() => ""));
    const scope = dialog.locator(".nd-setting-note").filter({ hasText: "Supported setup" });
    check.scopeLabel = compactCalculationText(await scope.locator("span").textContent().catch(() => ""));
    check.scopeDetail = compactCalculationText(await scope.locator("strong").textContent().catch(() => ""));
    const missingData = dialog.locator(".nd-setting-note").filter({ hasText: "Missing data" });
    check.missingDataLabel = compactCalculationText(await missingData.locator("span").textContent().catch(() => ""));
    check.missingDataDetail = compactCalculationText(await missingData.locator("strong").textContent().catch(() => ""));

    const pcaWeighting = dialog.locator('#nd-calculation-weighting option[value="pca"]');
    check.pcaWeightingOptionCount = await pcaWeighting.count();
    check.pcaWeightingDisabled = check.pcaWeightingOptionCount === 1
      && await pcaWeighting.evaluate((option) => option.disabled).catch(() => false);
    check.maximumIterationsCount = await dialog.getByLabel("Maximum iterations", { exact: true }).count();
    check.toleranceCount = await dialog.getByLabel("Stop criterion", { exact: true }).count();
    check.resamplingControlCount = await dialog.locator([
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
    ].join(", ")).count();
    check.caseWeightControlCount = await dialog.locator("#nd-calculation-case-weight").count();
    const start = dialog.getByRole("button", { name: "Start composite diagnostics", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.readinessText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.previewRuntimeBlockerVisible = check.readinessText.includes("Calculations require the offline QuickPLS desktop runtime");
    check.ccaModelScopeBlockerAbsent = !/require at least two constructs|require reflective constructs|require at least one structural path|do not support control paths|do not support interaction or higher-order|require path or factor weighting|require standardized preprocessing|do not support case weights|do not calculate resampling inference/i.test(check.readinessText);
    const selectedPanelText = compactCalculationText(await dialog.locator("#nd-calculation-panel").textContent().catch(() => ""));
    check.descriptiveBoundaryVisible = selectedPanelText.includes("descriptive residual diagnostics only")
      && !/threshold|pass\/fail|fit classification|p-value|confidence interval/i.test(selectedPanelText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "cca-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.cca.push(check);
    recordSkip("cca-completed-results-browser", "Completed CCA results are not synthesized in the browser visual harness. A genuine packaged-Tauri calculation, residual result, export, save, and reopen remain required.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the deterministic non-saturated X -> Z -> Y CCA fixture through the packaged Tauri application and capture the authoritative lifecycle and result evidence.",
    });
  }

  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("cca-method-editor-linkage", "CCA composite residual diagnostics did not open as the selected, labelled method editor at " + viewport.id + ".", check);
  }
  if (check.resultDataLabel !== "Result data" || check.resultDataDetail !== "Standardized (fixed)"
    || check.scopeLabel !== "Supported setup"
    || check.scopeDetail !== "Reflective composite path model; descriptive residual diagnostics only"
    || check.missingDataLabel !== "Missing data" || check.missingDataDetail !== "Listwise deletion") {
    recordFailure("cca-bounded-scope", "The CCA editor did not disclose its exact standardized, listwise, descriptive-only scope at " + viewport.id + ".", check);
  }
  if (check.pcaWeightingOptionCount !== 1 || !check.pcaWeightingDisabled
    || check.maximumIterationsCount !== 1 || check.toleranceCount !== 1
    || check.resamplingControlCount !== 0 || check.caseWeightControlCount !== 0) {
    recordFailure("cca-settings-contract", "The CCA editor exposed unsupported settings or omitted required bounded settings at " + viewport.id + ".", check);
  }
  if (check.startCommandCount !== 1 || !check.startCommandDisabled
    || !check.previewRuntimeBlockerVisible || !check.ccaModelScopeBlockerAbsent
    || !check.descriptiveBoundaryVisible) {
    recordFailure("cca-command-and-readiness-boundary", "The CCA editor did not expose one truthful composite-diagnostics command, a valid model scope blocked only by the browser runtime, and a threshold-free descriptive boundary at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("cca-dialog-truth-layout", "The idle CCA editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("cca-dialog-close-focus", "Close did not dismiss CCA and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditCtaPlsDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    category: "",
    scopeLabel: "",
    eligibleBlockSummary: "",
    scopeDisclosure: "",
    descriptiveOnlyBoundary: false,
    pcaWeightingOptionCount: 0,
    pcaWeightingDisabled: false,
    resamplingControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    readinessText: "",
    missingEligibleBlockerVisible: false,
    previewRuntimeBlockerVisible: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "cta-pls-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "cta_pls");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;
    check.category = compactCalculationText(await dialog.locator("#nd-calculation-category-assessment").textContent().catch(() => ""));

    const scope = dialog.locator("#nd-calculation-cta-pls-scope");
    check.scopeLabel = compactCalculationText(await scope.locator("span").textContent().catch(() => ""));
    check.eligibleBlockSummary = compactCalculationText(await scope.locator("strong").textContent().catch(() => ""));
    check.scopeDisclosure = compactCalculationText(await scope.locator("small").textContent().catch(() => ""));
    check.descriptiveOnlyBoundary = /descriptive sample-covariance tetrads only/i.test(check.scopeDisclosure)
      && /all three pairings for every four-indicator subset/i.test(check.scopeDisclosure)
      && /does not classify blocks/i.test(check.scopeDisclosure)
      && /does not .*bootstrap, permutation, asymptotic, or vanishing-tetrad decisions/i.test(check.scopeDisclosure);

    const pcaWeighting = dialog.locator('#nd-calculation-weighting option[value="pca"]');
    check.pcaWeightingOptionCount = await pcaWeighting.count();
    check.pcaWeightingDisabled = check.pcaWeightingOptionCount === 1
      && await pcaWeighting.evaluate((option) => option.disabled).catch(() => false);
    check.resamplingControlCount = await dialog.locator([
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start tetrad diagnostics", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.readinessText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.missingEligibleBlockerVisible = /CTA-PLS requires at least one ordinary construct with four or more assigned indicators/i.test(check.readinessText);
    check.previewRuntimeBlockerVisible = check.readinessText.includes("Calculations require the offline QuickPLS desktop runtime");
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "cta-pls-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.ctaPls.push(check);
    recordSkip("cta-pls-completed-results-browser", "The browser visual harness verifies CTA-PLS discovery and truthful invalid-model setup only. Genuine tetrad calculation, export, persistence, and tamper evidence remain authoritative in the CTA-PLS method factory.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Consume the native-qualified CTA-PLS factory evidence; do not synthesize completed tetrad results in this browser harness.",
    });
  }

  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage || check.category !== "Assessment") {
    recordFailure("cta-pls-method-editor-linkage", "Confirmatory Tetrad Analysis did not open as the selected, labelled Assessment editor at " + viewport.id + ".", check);
  }
  if (check.scopeLabel !== "Eligible indicator blocks"
    || check.eligibleBlockSummary !== "None - assign at least four indicators to one ordinary construct"
    || !check.descriptiveOnlyBoundary) {
    recordFailure("cta-pls-bounded-scope", "CTA-PLS did not disclose its exact descriptive-only scope and the sample model's ineligible indicator blocks at " + viewport.id + ".", check);
  }
  if (check.pcaWeightingOptionCount !== 1 || !check.pcaWeightingDisabled || check.resamplingControlCount !== 0) {
    recordFailure("cta-pls-settings-contract", "CTA-PLS exposed unsupported PCA weighting or inferential resampling controls at " + viewport.id + ".", check);
  }
  if (check.startCommandCount !== 1 || !check.startCommandDisabled
    || !check.missingEligibleBlockerVisible || !check.previewRuntimeBlockerVisible) {
    recordFailure("cta-pls-invalid-model-boundary", "CTA-PLS did not expose one disabled tetrad command with explicit eligible-block and desktop-runtime blockers at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("cta-pls-dialog-truth-layout", "The idle CTA-PLS editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("cta-pls-dialog-close-focus", "Close did not dismiss CTA-PLS and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditCbsemDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    modelType: "",
    modelTypeOptions: [],
    weighting: "",
    resultData: "",
    estimator: "",
    scope: "",
    bootstrapStatus: "",
    bootstrapToggleCount: 0,
    bootstrapToggleChecked: false,
    missingData: "",
    maximumIterationsCount: 0,
    toleranceCount: 0,
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    readinessText: "",
    previewRuntimeBlockerVisible: false,
    fiveCaseSampleBlockerVisible: false,
    structuralScopeBlockerAbsent: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "cbsem-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "cbsem");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;
    const modelType = dialog.locator("#nd-calculation-cbsem-model-type");
    check.modelType = await modelType.inputValue();
    check.modelTypeOptions = await modelType.locator("option").allTextContents();
    check.weighting = compactCalculationText(await dialog.locator(".nd-setting-note").filter({ hasText: "Weighting scheme" }).locator("strong").textContent().catch(() => ""));
    check.resultData = compactCalculationText(await dialog.locator(".nd-setting-note").filter({ hasText: "Result data" }).locator("strong").textContent().catch(() => ""));
    check.estimator = compactCalculationText(await dialog.locator("#nd-calculation-cbsem-estimator strong").textContent().catch(() => ""));
    check.scope = compactCalculationText(await dialog.locator("#nd-calculation-cbsem-scope strong").textContent().catch(() => ""));
    check.bootstrapStatus = compactCalculationText(await dialog.locator("#nd-calculation-cbsem-bootstrap-status strong").textContent().catch(() => ""));
    const bootstrapToggle = dialog.locator("#nd-calculation-cbsem-bootstrap-enabled");
    check.bootstrapToggleCount = await bootstrapToggle.count();
    check.bootstrapToggleChecked = check.bootstrapToggleCount === 1 && await bootstrapToggle.isChecked();
    check.missingData = compactCalculationText(await dialog.locator(".nd-setting-note").filter({ hasText: "Missing data" }).locator("strong").textContent().catch(() => ""));
    check.maximumIterationsCount = await dialog.locator("#nd-calculation-max-iterations").count();
    check.toleranceCount = await dialog.locator("#nd-calculation-tolerance").count();
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-case-weight",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start CB-SEM / CFA", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.readinessText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.previewRuntimeBlockerVisible = check.readinessText.includes("Calculations require the offline QuickPLS desktop runtime");
    check.fiveCaseSampleBlockerVisible = /requires at least 10 observations/i.test(check.readinessText)
      && /requires at least 10 complete cases/i.test(check.readinessText)
      && /5 remain/i.test(check.readinessText);
    check.structuralScopeBlockerAbsent = !/measurement-only model|requires at least one recursive latent path|requires at least one latent factor|requires reflective factors|requires at least two indicators|does not support control|does not support interaction/i.test(check.readinessText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "cbsem-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.cbsem.push(check);
    recordSkip("cbsem-completed-results-browser", "Completed CB-SEM/CFA results are not synthesized in the browser visual harness. A genuine packaged-Tauri calculation, ML result, export, save, and reopen remain required.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the deterministic 240-case X -> M -> Y reflective SEM fixture through the packaged Tauri application and capture its real ML lifecycle and result evidence.",
    });
  }

  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("cbsem-method-editor-linkage", "CB-SEM / CFA did not open as the selected, labelled method editor at " + viewport.id + ".", check);
  }
  if (check.modelType !== "sem"
    || JSON.stringify(check.modelTypeOptions) !== JSON.stringify(["Structural equation model (paths required)", "Confirmatory factor analysis (no paths)"])
    || check.weighting !== "Path weighting (fixed)" || check.resultData !== "Standardized (fixed)"
    || !/Maximum likelihood; first loading fixed to 1/i.test(check.estimator)
    || !/Single-group reflective raw-data CFA or recursive SEM/i.test(check.scope)
    || !/listwise-standardized indicators/i.test(check.scope)
    || !/no mean structure, robust\/ordinal\/FIML estimator, or invariance testing/i.test(check.scope)
    || !/Exact CFA case bootstrap is available from the Exact CB-SEM model tab/i.test(check.bootstrapStatus)
    || !/Historical schema-3 v2 and analytical v1 bootstrap results remain readable/i.test(check.bootstrapStatus)
    || !/cannot be selected for a new calculation/i.test(check.bootstrapStatus)
    || check.bootstrapToggleCount !== 0
    || check.bootstrapToggleChecked
    || check.missingData !== "Listwise deletion") {
    recordFailure("cbsem-bounded-scope", "CB-SEM / CFA did not disclose its exact single-group reflective ML and marker-identification scope at " + viewport.id + ".", check);
  }
  if (check.maximumIterationsCount !== 1 || check.toleranceCount !== 1 || check.unsupportedControlCount !== 0) {
    recordFailure("cbsem-settings-contract", "CB-SEM / CFA exposed unsupported settings or omitted its optimizer controls at " + viewport.id + ".", check);
  }
  if (check.startCommandCount !== 1 || !check.startCommandDisabled || !check.previewRuntimeBlockerVisible
    || !check.fiveCaseSampleBlockerVisible || !check.structuralScopeBlockerAbsent) {
    recordFailure("cbsem-command-and-readiness-boundary", "CB-SEM / CFA did not truthfully block the five-case browser sample while preserving a structurally eligible SEM model at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("cbsem-dialog-truth-layout", "The idle CB-SEM / CFA editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("cbsem-dialog-close-focus", "Close did not dismiss CB-SEM / CFA and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditIpmaDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    targetControlCount: 0,
    targetPlaceholder: "",
    targetOptions: [],
    selectedTargetId: "",
    selectedTargetLabel: "",
    weightingDetail: "",
    resultDataDetail: "",
    missingDataDetail: "",
    reportedConstructsDetail: "",
    performanceScopeDetail: "",
    maximumIterationsCount: 0,
    toleranceCount: 0,
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    readinessText: "",
    previewRuntimeBlockerVisible: false,
    predecessorOnlyDisclosureVisible: false,
    observedRangeDisclosureVisible: false,
    noCipmaOrInferenceClaim: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "ipma-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "ipma");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;

    const target = dialog.locator("#nd-calculation-ipma-target");
    check.targetControlCount = await target.count();
    check.targetOptions = await target.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    })));
    check.targetPlaceholder = check.targetOptions.find((option) => option.value === "")?.label ?? "";
    await target.selectOption("satisfaction");
    check.selectedTargetId = await target.inputValue();
    check.selectedTargetLabel = compactCalculationText(await target.locator("option:checked").textContent().catch(() => ""));

    const noteValue = async (label) => compactCalculationText(await dialog.locator(".nd-setting-note")
      .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
    check.weightingDetail = await noteValue("Weighting scheme");
    check.resultDataDetail = await noteValue("Result data");
    check.missingDataDetail = await noteValue("Missing data");
    check.reportedConstructsDetail = await noteValue("Reported constructs");
    check.performanceScopeDetail = await noteValue("Performance definition");
    check.maximumIterationsCount = await dialog.getByLabel("Maximum iterations", { exact: true }).count();
    check.toleranceCount = await dialog.getByLabel("Stop criterion", { exact: true }).count();
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start importance-performance analysis", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.readinessText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.previewRuntimeBlockerVisible = /offline QuickPLS desktop runtime|cannot run/i.test(check.readinessText);
    check.predecessorOnlyDisclosureVisible = check.reportedConstructsDetail
      === "Direct and indirect structural predecessors only; the target and unrelated constructs are omitted";
    check.observedRangeDisclosureVisible = check.performanceScopeDetail.includes("observed-range scaling of standardized composite scores; no theoretical-range correction");
    const selectedPanelText = compactCalculationText(await dialog.locator("#nd-calculation-panel").textContent().catch(() => ""));
    check.noCipmaOrInferenceClaim = !/\bcIPMA\b|p[- ]?value|confidence interval|bootstrap|permutation|resampling inference|significance/i.test(selectedPanelText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "ipma-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.ipma.push(check);
    recordSkip("ipma-completed-results-browser", "Completed IPMA results are not synthesized in the browser visual harness. A genuine packaged-Tauri calculation, predecessor-only tables and plot, export, save, and reopen remain required.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the tracked X/Z/M/Y IPMA fixture with Y selected through the packaged Tauri application and capture authoritative lifecycle and result evidence.",
    });
  }

  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("ipma-method-editor-linkage", "Importance-Performance Map Analysis did not open as the selected, labelled method editor at " + viewport.id + ".", check);
  }
  if (check.targetControlCount !== 1
    || check.targetPlaceholder !== "Select one endogenous construct"
    || check.selectedTargetId !== "satisfaction"
    || check.selectedTargetLabel !== "Customer Satisfaction [satisfaction]") {
    recordFailure("ipma-explicit-endogenous-target", "IPMA did not require and retain the explicit immutable Customer Satisfaction target at " + viewport.id + ".", check);
  }
  if (check.weightingDetail !== "Path weighting (fixed)"
    || check.resultDataDetail !== "Standardized (fixed)"
    || check.missingDataDetail !== "Listwise deletion"
    || check.maximumIterationsCount !== 1
    || check.toleranceCount !== 1
    || check.unsupportedControlCount !== 0) {
    recordFailure("ipma-fixed-settings-contract", "IPMA did not retain its fixed path-weighted, standardized, listwise contract without resampling or inference controls at " + viewport.id + ".", check);
  }
  if (!check.predecessorOnlyDisclosureVisible || !check.observedRangeDisclosureVisible || !check.noCipmaOrInferenceClaim) {
    recordFailure("ipma-bounded-scope-disclosure", "IPMA did not disclose predecessor-only observed-range performance without cIPMA or inferential claims at " + viewport.id + ".", check);
  }
  if (check.startCommandCount !== 1 || !check.startCommandDisabled || !check.previewRuntimeBlockerVisible) {
    recordFailure("ipma-browser-runtime-boundary", "IPMA did not expose one truthfully blocked desktop-only start command in browser preview at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("ipma-dialog-truth-layout", "The idle IPMA editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("ipma-dialog-close-focus", "Close did not dismiss IPMA and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditNcaStandaloneDialogFromData(page, viewport, sequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    dataSurface: false,
    dataColumns: [],
    visibleModelNodes: null,
    analyzeCommandCount: 0,
    calculateCommandCount: 0,
    dialogOpened: false,
    selectedMethod: "",
    linkage: null,
    catalogCount: 0,
    category: "",
    xOptions: [],
    yOptions: [],
    selectedX: "",
    selectedY: "",
    ceilingModes: [],
    ceilingSelections: [],
    permutations: null,
    seed: null,
    variableData: "",
    validatedScope: "",
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    runtimeBlockerVisible: false,
    noModelBlocker: false,
    truthAndOverflow: null,
    closeFocus: null,
    noFabricatedResult: false,
  };
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  let trigger = null;
  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadNcaFixture === "function");
    if (!check.fixtureApiPresent) {
      recordFailure("nca-browser-fixture-api", `The data-only NCA smoke fixture was not exposed at ${viewport.id}.`, check);
      return;
    }
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadNcaFixture());
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dataSurface = await page.locator('.nd-app[data-surface="data"]').count() === 1;
    check.dataColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactCalculationText);
    check.visibleModelNodes = await page.locator(".react-flow__node-latent").count();

    trigger = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
    check.analyzeCommandCount = await trigger.count();
    check.calculateCommandCount = await page.locator('.nd-commandbar[role="toolbar"] button')
      .filter({ hasText: /^Calculate(?:\u2026|\.\.\.)?$/i }).count();
    if (check.analyzeCommandCount !== 1) {
      recordFailure("nca-data-analyze-command", `Data did not expose exactly one Analyze command at ${viewport.id}.`, check);
      return;
    }
    await trigger.focus();
    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dialogOpened = await dialog.isVisible().catch(() => false);
    if (!check.dialogOpened) {
      recordFailure("nca-data-analyze-dialog", `The Data Analyze command did not open the shared calculation dialog at ${viewport.id}.`, check);
      return;
    }

    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    check.catalogCount = await listbox.getByRole("option").count();
    const ncaOption = calculationOption(dialog, "nca");
    if (await ncaOption.getAttribute("aria-selected") !== "true") await ncaOption.click();
    check.selectedMethod = compactCalculationText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent().catch(() => ""));
    check.linkage = await inspectSelectedMethodLinkage(dialog, "nca");
    check.category = compactCalculationText(await dialog.locator("#nd-calculation-category-standalone").textContent().catch(() => ""));

    const x = dialog.locator("#nd-calculation-nca-x");
    const y = dialog.locator("#nd-calculation-nca-y");
    check.xOptions = await x.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
      disabled: option.disabled,
    })));
    await x.selectOption("condition");
    check.yOptions = await y.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
      disabled: option.disabled,
    })));
    await y.selectOption("outcome");
    check.selectedX = await x.inputValue();
    check.selectedY = await y.inputValue();

    const ceiling = dialog.locator("#nd-calculation-nca-ceiling");
    check.ceilingModes = await ceiling.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    })));
    for (const mode of ["ce_fdh", "cr_fdh", "both"]) {
      await ceiling.selectOption(mode);
      check.ceilingSelections.push(await ceiling.inputValue());
    }
    const permutations = dialog.locator("#nd-calculation-nca-permutations");
    await permutations.fill("19");
    check.permutations = {
      value: await permutations.inputValue(),
      min: await permutations.getAttribute("min"),
      max: await permutations.getAttribute("max"),
      step: await permutations.getAttribute("step"),
    };
    const seed = dialog.locator("#nd-calculation-seed");
    await seed.fill("20260811");
    check.seed = { value: await seed.inputValue(), min: await seed.getAttribute("min"), max: await seed.getAttribute("max") };
    const noteValue = async (label) => compactCalculationText(await dialog.locator(".nd-setting-note")
      .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
    check.variableData = await noteValue("Variable data");
    check.validatedScope = await noteValue("Supported setup");
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
      "#nd-calculation-group-column",
      "#nd-calculation-ipma-target",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start necessary condition analysis", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    const blockerText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.runtimeBlockerVisible = /offline QuickPLS desktop runtime|cannot run/i.test(blockerText);
    check.noModelBlocker = !/construct|structural path|editable model|active model/i.test(blockerText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "nca-standalone-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);

    await setSurface(page, "results");
    await page.getByText("No completed calculation", { exact: true }).waitFor({ timeout: 1_000 }).catch(() => null);
    check.noFabricatedResult = await page.getByText("No completed calculation", { exact: true }).count() === 1
      && await page.locator(".nd-result-table, .nd-result-diagram-view, .nd-run-select option").count() === 0;
    await capture(page, "nca-empty-results", sequence + 1, viewport);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.nca.push(check);
    recordSkip("nca-completed-results-browser", "The browser preview inspects the truthful standalone setup but does not synthesize NCA results. Genuine CE-FDH/CR-FDH geometry, lifecycle, export, save, and reopen remain packaged-Tauri checks.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the deterministic x/y NCA fixture with no editable model through the packaged Tauri application.",
    });
  }

  const expectedXOptions = [
    { value: "", label: "Select a numeric variable", disabled: false },
    { value: "condition", label: "condition", disabled: false },
    { value: "outcome", label: "outcome", disabled: false },
  ];
  const expectedYOptions = [
    { value: "", label: "Select a different numeric variable", disabled: false },
    { value: "condition", label: "condition", disabled: true },
    { value: "outcome", label: "outcome", disabled: false },
  ];
  const expectedCeilingModes = [
    { value: "both", label: "CE-FDH and CR-FDH" },
    { value: "ce_fdh", label: "CE-FDH" },
    { value: "cr_fdh", label: "CR-FDH" },
  ];
  if (JSON.stringify(check.fixture) !== JSON.stringify({ variables: 2, models: 0 })
    || !check.dataSurface
    || !["condition", "outcome"].every((column) => check.dataColumns.includes(column))
    || check.visibleModelNodes !== 0) {
    recordFailure("nca-data-only-fixture", `NCA did not launch from the deterministic two-variable Data workspace with zero editable models at ${viewport.id}.`, check);
  }
  if (check.analyzeCommandCount !== 1 || check.calculateCommandCount !== 0 || !check.dialogOpened
    || check.selectedMethod !== "Necessary Condition Analysis" || !check.linkage?.linkage
    || check.catalogCount !== nativeCalculationMethods.length || check.category !== "Standalone analysis") {
    recordFailure("nca-data-analyze-command", `Data did not expose one shared Analyze command opening the ${nativeCalculationMethods.length}-method catalog with NCA selected at ${viewport.id}.`, check);
  }
  if (JSON.stringify(check.xOptions) !== JSON.stringify(expectedXOptions)
    || JSON.stringify(check.yOptions) !== JSON.stringify(expectedYOptions)
    || check.selectedX !== "condition" || check.selectedY !== "outcome") {
    recordFailure("nca-numeric-variable-contract", `NCA did not expose distinct numeric X/Y selectors bound to the data-only fixture at ${viewport.id}.`, check);
  }
  if (JSON.stringify(check.ceilingModes) !== JSON.stringify(expectedCeilingModes)
    || JSON.stringify(check.ceilingSelections) !== JSON.stringify(["ce_fdh", "cr_fdh", "both"])
    || check.permutations?.value !== "19" || check.permutations?.min !== "1"
    || check.permutations?.max !== "10000" || check.permutations?.step !== "1"
    || check.seed?.value !== "20260811" || check.seed?.min !== "0" || check.seed?.max !== "4294967295") {
    recordFailure("nca-ceiling-resampling-contract", `NCA did not retain CE-FDH/CR-FDH/both plus bounded seeded internal permutations at ${viewport.id}.`, check);
  }
  if (check.variableData !== "Observed numeric values (fixed)"
    || check.validatedScope !== nativeNcaScopeNote || check.unsupportedControlCount !== 0
    || check.startCommandCount !== 1 || !check.startCommandDisabled
    || !check.runtimeBlockerVisible || !check.noModelBlocker) {
    recordFailure("nca-bounded-scope-contract", `NCA did not disclose its exact standalone observed-variable scope without model or unsupported controls at ${viewport.id}.`, check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow
    || !check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored || !check.noFabricatedResult) {
    recordFailure("nca-browser-truth-layout", `The idle standalone NCA workflow exposed fabricated results/run state, overflow, or broken close focus at ${viewport.id}.`, check);
  }
}

async function auditPcaStandaloneDialogFromData(page, viewport, sequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    dataSurface: false,
    dataColumns: [],
    visibleModelNodes: null,
    analyzeCommandCount: 0,
    calculateCommandCount: 0,
    dialogOpened: false,
    selectedMethod: "",
    linkage: null,
    catalogCount: 0,
    category: "",
    variableOptions: [],
    initialSelectedVariables: [],
    selectionActions: null,
    retentionModes: [],
    retentionSelections: [],
    fixedComponents: null,
    varianceThreshold: null,
    calculationBasis: "",
    variableData: "",
    validatedScope: "",
    maxIterationsCount: 0,
    toleranceCount: 0,
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    runtimeBlockerVisible: false,
    noModelBlocker: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  let trigger = null;
  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadPcaFixture === "function");
    if (!check.fixtureApiPresent) {
      recordFailure("pca-browser-fixture-api", `The data-only PCA smoke fixture was not exposed at ${viewport.id}.`, check);
      return;
    }
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadPcaFixture());
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dataSurface = await page.locator('.nd-app[data-surface="data"]').count() === 1;
    check.dataColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactCalculationText);
    check.visibleModelNodes = await page.locator(".react-flow__node-latent").count();

    trigger = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
    check.analyzeCommandCount = await trigger.count();
    check.calculateCommandCount = await page.locator('.nd-commandbar[role="toolbar"] button')
      .filter({ hasText: /^Calculate(?:\u2026|\.\.\.)?$/i }).count();
    if (check.analyzeCommandCount !== 1) {
      recordFailure("pca-data-analyze-command", `Data did not expose exactly one Analyze command at ${viewport.id}.`, check);
      return;
    }
    await trigger.focus();
    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dialogOpened = await dialog.isVisible().catch(() => false);
    if (!check.dialogOpened) {
      recordFailure("pca-data-analyze-dialog", `The Data Analyze command did not open the shared calculation dialog at ${viewport.id}.`, check);
      return;
    }

    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    check.catalogCount = await listbox.getByRole("option").count();
    const pcaOption = calculationOption(dialog, "pca");
    if (await pcaOption.getAttribute("aria-selected") !== "true") await pcaOption.click();
    await dialog.locator(".nd-pca-variables legend").filter({ hasText: "4 selected" })
      .waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.selectedMethod = compactCalculationText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent().catch(() => ""));
    check.linkage = await inspectSelectedMethodLinkage(dialog, "pca");
    check.category = compactCalculationText(await dialog.locator("#nd-calculation-category-standalone").textContent().catch(() => ""));

    const variableLabels = dialog.locator(".nd-pca-variable-list label");
    check.variableOptions = await variableLabels.evaluateAll((labels) => labels.map((label) => ({
      name: label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      checked: Boolean(label.querySelector('input[type="checkbox"]')?.checked),
      disabled: Boolean(label.querySelector('input[type="checkbox"]')?.disabled),
    })));
    check.initialSelectedVariables = check.variableOptions.filter((entry) => entry.checked).map((entry) => entry.name);
    const clear = dialog.getByRole("button", { name: "Clear", exact: true });
    const selectAll = dialog.getByRole("button", { name: "Select all numeric", exact: true });
    await clear.click();
    const selectedAfterClear = await variableLabels.locator('input[type="checkbox"]:checked').count();
    await selectAll.click();
    const selectedAfterSelectAll = await variableLabels.locator('input[type="checkbox"]:checked').count();
    check.selectionActions = { selectedAfterClear, selectedAfterSelectAll };

    const retention = dialog.locator("#nd-calculation-pca-rule");
    check.retentionModes = await retention.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    })));
    await retention.selectOption("fixed");
    check.retentionSelections.push(await retention.inputValue());
    const fixedComponents = dialog.locator("#nd-calculation-pca-components");
    await fixedComponents.fill("2");
    check.fixedComponents = {
      value: await fixedComponents.inputValue(),
      min: await fixedComponents.getAttribute("min"),
      max: await fixedComponents.getAttribute("max"),
      step: await fixedComponents.getAttribute("step"),
    };
    await retention.selectOption("variance_threshold");
    check.retentionSelections.push(await retention.inputValue());
    const threshold = dialog.locator("#nd-calculation-pca-threshold");
    await threshold.fill("80");
    check.varianceThreshold = {
      value: await threshold.inputValue(),
      min: await threshold.getAttribute("min"),
      max: await threshold.getAttribute("max"),
      step: await threshold.getAttribute("step"),
    };
    await retention.selectOption("kaiser");
    check.retentionSelections.push(await retention.inputValue());

    const noteValue = async (label) => compactCalculationText(await dialog.locator(".nd-setting-note")
      .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
    check.calculationBasis = await noteValue("Calculation basis");
    check.variableData = await noteValue("Variable data");
    check.validatedScope = await noteValue("Validated scope");
    check.maxIterationsCount = await dialog.locator("#nd-calculation-max-iterations").count();
    check.toleranceCount = await dialog.locator("#nd-calculation-tolerance").count();
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-nca-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
      "#nd-calculation-group-column",
      "#nd-calculation-ipma-target",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start principal component analysis", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    const blockerText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.runtimeBlockerVisible = /offline QuickPLS desktop runtime|cannot run/i.test(blockerText);
    check.noModelBlocker = !/construct|structural path|editable model|active model/i.test(blockerText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "pca-standalone-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.pca.push(check);
    recordSkip("pca-completed-results-browser", "The browser preview inspects the truthful standalone PCA setup but does not synthesize PCA results. Genuine eigendecomposition, score export, lifecycle, save, and reopen remain packaged-Tauri checks.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the deterministic numeric PCA fixture with no editable model through the packaged Tauri application.",
    });
  }

  const expectedVariables = ["service", "quality", "value", "trust"];
  const expectedRetentionModes = [
    { value: "kaiser", label: "Kaiser criterion (eigenvalue at least 1)" },
    { value: "fixed", label: "Fixed component count" },
    { value: "variance_threshold", label: "Cumulative variance threshold" },
  ];
  if (JSON.stringify(check.fixture) !== JSON.stringify({ variables: 5, models: 0 })
    || !check.dataSurface
    || ![...expectedVariables, "segment"].every((column) => check.dataColumns.includes(column))
    || check.visibleModelNodes !== 0) {
    recordFailure("pca-data-only-fixture", `PCA did not launch from the deterministic five-variable Data workspace with zero editable models at ${viewport.id}.`, check);
  }
  if (check.analyzeCommandCount !== 1 || check.calculateCommandCount !== 0 || !check.dialogOpened
    || check.selectedMethod !== "Principal Component Analysis" || !check.linkage?.linkage
    || check.catalogCount !== nativeCalculationMethods.length || check.category !== "Standalone analysis") {
    recordFailure("pca-data-analyze-command", `Data did not expose one shared Analyze command opening the ${nativeCalculationMethods.length}-method catalog with PCA selected at ${viewport.id}.`, check);
  }
  if (JSON.stringify(check.variableOptions.map((entry) => entry.name)) !== JSON.stringify(expectedVariables)
    || JSON.stringify(check.initialSelectedVariables) !== JSON.stringify(expectedVariables)
    || check.variableOptions.some((entry) => entry.disabled)
    || check.selectionActions?.selectedAfterClear !== 0 || check.selectionActions?.selectedAfterSelectAll !== 4) {
    recordFailure("pca-numeric-variable-contract", `PCA did not expose only the four numeric fixture variables with working Select all/Clear actions at ${viewport.id}.`, check);
  }
  if (JSON.stringify(check.retentionModes) !== JSON.stringify(expectedRetentionModes)
    || JSON.stringify(check.retentionSelections) !== JSON.stringify(["fixed", "variance_threshold", "kaiser"])
    || check.fixedComponents?.value !== "2" || check.fixedComponents?.min !== "1"
    || check.fixedComponents?.max !== "4" || check.fixedComponents?.step !== "1"
    || check.varianceThreshold?.value !== "80" || check.varianceThreshold?.min !== "1"
    || check.varianceThreshold?.max !== "99.9" || check.varianceThreshold?.step !== "0.1") {
    recordFailure("pca-retention-contract", `PCA did not expose exact Kaiser, fixed-count, and variance-threshold retention controls at ${viewport.id}.`, check);
  }
  if (check.calculationBasis !== "Correlation matrix (fixed)"
    || check.variableData !== "Standardized numeric values (fixed)"
    || check.validatedScope !== nativePcaScopeNote
    || check.maxIterationsCount !== 0 || check.toleranceCount !== 0
    || check.unsupportedControlCount !== 0
    || check.startCommandCount !== 1 || !check.startCommandDisabled
    || !check.runtimeBlockerVisible || !check.noModelBlocker) {
    recordFailure("pca-bounded-scope-contract", `PCA did not disclose its exact standalone standardized correlation-matrix scope without model or unsupported controls at ${viewport.id}.`, check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow
    || !check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("pca-browser-truth-layout", `The idle standalone PCA workflow exposed fabricated run state, overflow, or broken close focus at ${viewport.id}.`, check);
  }
}

async function auditOlsStandaloneDialogFromData(page, viewport, sequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    dataSurface: false,
    dataColumns: [],
    visibleModelNodes: null,
    analyzeCommandCount: 0,
    dialogOpened: false,
    catalogCount: 0,
    selectedMethod: "",
    linkage: null,
    category: "",
    outcomeOptions: [],
    outcome: "",
    predictorOptions: [],
    controlOptions: [],
    predictors: [],
    controls: [],
    calculationBasis: "",
    variableData: "",
    uncertainty: "",
    validatedScope: "",
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    runtimeBlockerVisible: false,
    noModelBlocker: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  let trigger = null;
  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadOlsFixture === "function");
    if (!check.fixtureApiPresent) {
      recordFailure("ols-browser-fixture-api", `The data-only OLS smoke fixture was not exposed at ${viewport.id}.`, check);
      return;
    }
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadOlsFixture());
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dataSurface = await page.locator('.nd-app[data-surface="data"]').count() === 1;
    check.dataColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactCalculationText);
    check.visibleModelNodes = await page.locator(".react-flow__node-latent").count();

    trigger = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
    check.analyzeCommandCount = await trigger.count();
    if (check.analyzeCommandCount !== 1) return;
    await trigger.focus();
    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dialogOpened = await dialog.isVisible().catch(() => false);
    if (!check.dialogOpened) return;

    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    check.catalogCount = await listbox.getByRole("option").count();
    const option = calculationOption(dialog, "regression");
    if (await option.getAttribute("aria-selected") !== "true") await option.click();
    const regressionType = dialog.locator("#nd-calculation-regression-type");
    await regressionType.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    if (await regressionType.inputValue().catch(() => "") !== "ols") await regressionType.selectOption("ols");
    check.selectedMethod = compactCalculationText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent().catch(() => ""));
    check.linkage = await inspectSelectedMethodLinkage(dialog, "regression");
    check.category = compactCalculationText(await dialog.locator("#nd-calculation-category-standalone").textContent().catch(() => ""));

    const outcome = dialog.locator("#nd-calculation-regression-outcome");
    check.outcomeOptions = await outcome.locator('option:not([value=""])').allTextContents();
    check.outcome = await outcome.inputValue();
    const roleFieldsets = dialog.locator(".nd-ols-settings fieldset.nd-pca-variables");
    const inspectRoles = async (fieldset) => fieldset.locator("label").evaluateAll((labels) => labels.map((label) => ({
      name: label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      checked: Boolean(label.querySelector('input[type="checkbox"]')?.checked),
      disabled: Boolean(label.querySelector('input[type="checkbox"]')?.disabled),
    })));
    check.predictorOptions = await inspectRoles(roleFieldsets.nth(0));
    check.controlOptions = await inspectRoles(roleFieldsets.nth(1));
    check.predictors = check.predictorOptions.filter((entry) => entry.checked).map((entry) => entry.name);
    const controlInput = roleFieldsets.nth(1).locator("label").filter({ hasText: /^control$/ }).locator('input[type="checkbox"]');
    await controlInput.check();
    check.controls = (await inspectRoles(roleFieldsets.nth(1))).filter((entry) => entry.checked).map((entry) => entry.name);

    const noteValue = async (label) => compactCalculationText(await dialog.locator(".nd-setting-note")
      .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
    check.calculationBasis = await noteValue("Calculation basis");
    check.variableData = await noteValue("Variable data");
    check.uncertainty = await noteValue("Uncertainty");
    check.validatedScope = await noteValue("Validated scope");
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start OLS regression", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    const blockerText = compactCalculationText(await dialog.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
    check.runtimeBlockerVisible = /offline QuickPLS desktop runtime|cannot run/i.test(blockerText);
    check.noModelBlocker = !/construct|structural path|editable model|active model/i.test(blockerText);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "ols-standalone-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    evidence.checks.ols.push(check);
    recordSkip("ols-completed-results-browser", "The browser preview inspects the truthful standalone OLS setup but does not synthesize OLS coefficients. Genuine HC3 inference, export, save, and reopen remain packaged-Tauri checks.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run a deterministic raw numeric OLS fixture with no editable model through the packaged Tauri application.",
    });
  }

  const expectedNumeric = ["outcome", "predictor", "moderator", "control"];
  if (JSON.stringify(check.fixture) !== JSON.stringify({ variables: 5, models: 0 })
    || !check.dataSurface || ![...expectedNumeric, "group"].every((column) => check.dataColumns.includes(column))
    || check.visibleModelNodes !== 0) {
    recordFailure("ols-data-only-fixture", `OLS did not launch from the deterministic five-variable Data workspace with zero editable models at ${viewport.id}.`, check);
  }
  if (check.analyzeCommandCount !== 1 || !check.dialogOpened || check.catalogCount !== nativeCalculationMethods.length
    || check.selectedMethod !== "Regression" || !check.linkage?.linkage || check.category !== "Standalone analysis") {
    recordFailure("ols-data-analyze-command", `Data did not open the shared ${nativeCalculationMethods.length}-method catalog with OLS selected at ${viewport.id}.`, check);
  }
  if (JSON.stringify(check.outcomeOptions.map(compactCalculationText)) !== JSON.stringify(expectedNumeric)
    || check.outcome !== "outcome"
    || JSON.stringify(check.predictorOptions.map((entry) => entry.name)) !== JSON.stringify(expectedNumeric)
    || JSON.stringify(check.controlOptions.map((entry) => entry.name)) !== JSON.stringify(expectedNumeric)
    || JSON.stringify(check.predictors) !== JSON.stringify(["predictor"])
    || JSON.stringify(check.controls) !== JSON.stringify(["control"])) {
    recordFailure("ols-variable-role-contract", `OLS did not preserve distinct numeric outcome, predictor, and optional-control selections at ${viewport.id}.`, check);
  }
  if (check.calculationBasis !== "Raw-value OLS with intercept (fixed)"
    || check.variableData !== "Unstandardized numeric values (fixed)"
    || check.uncertainty !== "HC3 robust SE; two-sided 95% CI (fixed)"
    || check.validatedScope !== nativeOlsScopeNote
    || check.unsupportedControlCount !== 0
    || check.startCommandCount !== 1 || !check.startCommandDisabled
    || !check.runtimeBlockerVisible || !check.noModelBlocker) {
    recordFailure("ols-bounded-scope-contract", `OLS did not disclose its fixed raw/listwise/HC3/95% standalone scope at ${viewport.id}.`, check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow
    || !check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("ols-browser-truth-layout", `The idle standalone OLS workflow exposed fabricated run state, overflow, or broken close focus at ${viewport.id}.`, check);
  }
}

async function auditLogisticStandaloneDialogFromData(page, viewport, sequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    dataSurface: false,
    visibleModelNodes: null,
    analyzeCommandCount: 0,
    dialogOpened: false,
    catalogCount: 0,
    selectedMethod: "",
    linkage: null,
    category: "",
    regressionTypeOptions: [],
    regressionType: "",
    outcome: "",
    predictors: [],
    controls: [],
    calculationBasis: "",
    variableData: "",
    uncertainty: "",
    profile: null,
    validatedScope: "",
    unsupportedControlCount: 0,
    startCommandCount: 0,
    startCommandDisabled: false,
    strictProfileBlockers: [],
    noModelBlocker: false,
    noPhantomResult: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  let trigger = null;
  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadOlsFixture === "function");
    if (!check.fixtureApiPresent) {
      recordFailure("logistic-browser-fixture-api", `The data-only numeric smoke fixture was not exposed at ${viewport.id}.`, check);
      return;
    }
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadOlsFixture());
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dataSurface = await page.locator('.nd-app[data-surface="data"]').count() === 1;
    check.visibleModelNodes = await page.locator(".react-flow__node-latent").count();

    trigger = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
    check.analyzeCommandCount = await trigger.count();
    if (check.analyzeCommandCount !== 1) return;
    await trigger.focus();
    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dialogOpened = await dialog.isVisible().catch(() => false);
    if (!check.dialogOpened) return;

    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    check.catalogCount = await listbox.getByRole("option").count();
    const option = calculationOption(dialog, "regression");
    if (await option.getAttribute("aria-selected") !== "true") await option.click();
    const regressionType = dialog.locator("#nd-calculation-regression-type");
    await regressionType.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    await regressionType.selectOption("logistic");
    check.selectedMethod = compactCalculationText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent().catch(() => ""));
    check.linkage = await inspectSelectedMethodLinkage(dialog, "regression");
    check.category = compactCalculationText(await dialog.locator("#nd-calculation-category-standalone").textContent().catch(() => ""));
    check.regressionTypeOptions = await regressionType.locator("option").evaluateAll((options) => options.map((entry) => ({
      value: entry.value,
      label: entry.textContent?.replace(/\s+/g, " ").trim() ?? "",
    })));
    check.regressionType = await regressionType.inputValue();
    const outcome = dialog.locator("#nd-calculation-regression-outcome");
    await outcome.selectOption("outcome");
    check.outcome = await outcome.inputValue();
    const predictorFieldset = dialog.getByRole("group", { name: /^Predictors \(\d+ selected\)$/ });
    const controlFieldset = dialog.getByRole("group", { name: /^Controls \(\d+ selected, optional\)$/ });
    const predictorInput = predictorFieldset.locator("label").filter({ hasText: /^predictor$/ }).locator('input[type="checkbox"]');
    if (!await predictorInput.isChecked()) await predictorInput.check();
    for (const selectedControl of await controlFieldset.locator('input[type="checkbox"]:checked').all()) {
      await selectedControl.uncheck();
    }
    await page.waitForFunction(() => {
      const fields = Array.from(document.querySelectorAll(".nd-ols-settings fieldset.nd-pca-variables"));
      const selectedPredictors = fields[0]?.querySelectorAll('input[type="checkbox"]:checked').length ?? -1;
      const selectedControls = fields[1]?.querySelectorAll('input[type="checkbox"]:checked').length ?? -1;
      return selectedPredictors === 1 && selectedControls === 0;
    }, null, { timeout: 1_000 });
    const inspectRoles = async (fieldset) => fieldset.locator("label").evaluateAll((labels) => labels.filter((label) => (
      label.querySelector('input[type="checkbox"]')?.checked
    )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
    check.predictors = await inspectRoles(predictorFieldset);
    check.controls = await inspectRoles(controlFieldset);
    const noteValue = async (label) => compactCalculationText(await dialog.locator(".nd-setting-note")
      .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
    check.calculationBasis = await noteValue("Calculation basis");
    check.variableData = await noteValue("Variable data");
    check.uncertainty = await noteValue("Uncertainty");
    const profile = dialog.locator("#nd-calculation-logistic-profile");
    check.profile = {
      text: compactCalculationText(await profile.textContent().catch(() => "")),
      role: await profile.getAttribute("role"),
      ariaLive: await profile.getAttribute("aria-live"),
      ariaBusy: await profile.getAttribute("aria-busy"),
    };
    check.validatedScope = await noteValue("Validated scope");
    check.unsupportedControlCount = await dialog.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance", "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight",
    ].join(", ")).count();
    const start = dialog.getByRole("button", { name: "Start binary logistic regression", exact: true });
    check.startCommandCount = await start.count();
    check.startCommandDisabled = check.startCommandCount === 1 && await start.isDisabled();
    check.strictProfileBlockers = (await dialog.locator(".nd-blocker li").allTextContents()).map(compactCalculationText);
    const blockerText = check.strictProfileBlockers.join(" ");
    check.noModelBlocker = !/construct|structural path|editable model|active model/i.test(blockerText);
    check.noPhantomResult = await page.locator(".nd-run-select select option").count() === 0;
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "logistic-standalone-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    evidence.checks.logistic.push(check);
    recordSkip("logistic-completed-results-browser", "The browser preview proves strict 0/1 readiness and responsive setup without synthesizing logistic output. Genuine ML estimation, lifecycle, tables, XLSX, save, archive inspection, and reopen remain packaged-Tauri checks.", {
      viewport: viewport.id,
      requiredNativeFollowUp: "Run the full 140-row binary fixture with no editable model through the focused packaged-Tauri logistic gate.",
    });
  }

  const expectedTypeOptions = [
    { value: "ols", label: "Ordinary least squares" },
    { value: "logistic", label: "Binary logistic (outcome coded 0/1)" },
    { value: "process", label: "Graph-defined Path Analysis / PROCESS" },
  ];
  if (JSON.stringify(check.fixture) !== JSON.stringify({ variables: 5, models: 0 })
    || !check.dataSurface || check.visibleModelNodes !== 0) {
    recordFailure("logistic-data-only-fixture", `Logistic setup did not begin from a numeric data-only workspace with zero editable models at ${viewport.id}.`, check);
  }
  if (check.analyzeCommandCount !== 1 || !check.dialogOpened || check.catalogCount !== nativeCalculationMethods.length
    || check.selectedMethod !== "Regression" || !check.linkage?.linkage || check.category !== "Standalone analysis") {
    recordFailure("logistic-data-analyze-command", `Data did not open the shared method catalog with Regression selected at ${viewport.id}.`, check);
  }
  if (JSON.stringify(check.regressionTypeOptions) !== JSON.stringify(expectedTypeOptions)
    || check.regressionType !== "logistic" || check.outcome !== "outcome"
    || JSON.stringify(check.predictors) !== JSON.stringify(["predictor"])
    || check.controls.length !== 0) {
    recordFailure("logistic-variable-role-contract", `Logistic setup did not preserve its explicit type, outcome, predictor, and optional-control roles at ${viewport.id}.`, check);
  }
  if (check.calculationBasis !== "Binary logistic maximum likelihood with intercept (fixed)"
    || check.variableData !== "Unstandardized numeric values (fixed)"
    || check.uncertainty !== "Maximum-likelihood SE; Wald z and two-sided 95% CI; odds ratios (fixed)"
    || check.validatedScope !== nativeLogisticScopeNote || check.unsupportedControlCount !== 0) {
    recordFailure("logistic-bounded-scope-contract", `Logistic setup did not disclose its exact raw/listwise/ML/Wald/odds-ratio scope at ${viewport.id}.`, check);
  }
  if (check.profile?.role !== "status" || check.profile?.ariaLive !== "polite" || check.profile?.ariaBusy !== "false"
    || !check.profile?.text.includes("36 complete cases: 0 class 0 and 0 class 1; 0 omitted by listwise deletion")
    || !check.strictProfileBlockers.some((message) => /not coded exactly 0 or 1/i.test(message))
    || !check.strictProfileBlockers.some((message) => /must contain both class 0 and class 1/i.test(message))
    || check.startCommandCount !== 1 || !check.startCommandDisabled || !check.noModelBlocker) {
    recordFailure("logistic-strict-profile-contract", `The full resident browser fixture was not truthfully rejected for failing exact numeric 0/1 outcome profiling at ${viewport.id}.`, check);
  }
  if (!check.noPhantomResult || !check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow
    || !check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("logistic-browser-truth-layout", `The idle rejected logistic setup exposed a phantom result, fabricated run state, overflow, or broken close focus at ${viewport.id}.`, check);
  }
}

async function auditRegressionBootstrapDialogFromData(page, viewport, olsSequence, logisticSequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    dataSurface: false,
    visibleModelNodes: null,
    analyzeCommandCount: 0,
    dialogOpened: false,
    catalogCount: 0,
    selectedMethod: "",
    linkage: null,
    category: "",
    regressionTypeOptions: [],
    outcome: "",
    roles: null,
    bootstrap: null,
    accessibility: null,
    ols: null,
    logistic: null,
    closeFocus: null,
    completedResult: null,
  };
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  let trigger = null;

  const inspectRoleFieldset = async (fieldset) => ({
    legend: compactCalculationText(await fieldset.locator("legend").textContent().catch(() => "")),
    variables: await fieldset.locator("label").evaluateAll((labels) => labels.map((label) => ({
      name: label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      checked: Boolean(label.querySelector('input[type="checkbox"]')?.checked),
      disabled: Boolean(label.querySelector('input[type="checkbox"]')?.disabled),
    }))),
  });
  const inspectRoles = async () => {
    const fieldsets = dialog.locator(".nd-ols-settings fieldset.nd-pca-variables");
    const predictors = await inspectRoleFieldset(fieldsets.nth(0));
    const controls = await inspectRoleFieldset(fieldsets.nth(1));
    return {
      fieldsetCount: await fieldsets.count(),
      predictors,
      controls,
      selectedPredictors: predictors.variables.filter((entry) => entry.checked).map((entry) => entry.name),
      selectedControls: controls.variables.filter((entry) => entry.checked).map((entry) => entry.name),
    };
  };
  const inspectNumberInput = async (input) => ({
    count: await input.count(),
    value: await input.inputValue().catch(() => ""),
    min: await input.getAttribute("min"),
    max: await input.getAttribute("max"),
    step: await input.getAttribute("step"),
  });
  const explicitLabelAssociationCount = (control) => control.evaluate((node) => (
    Array.from(node.labels ?? []).filter((label) => label.htmlFor === node.id).length
  ));
  const readSettingNote = async (label) => compactCalculationText(await dialog.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const inspectBlockers = async (allowedProfilePatterns = []) => {
    const messages = (await dialog.locator(".nd-blocker li").allTextContents()).map(compactCalculationText);
    const isRuntime = (message) => /Calculations require the offline QuickPLS desktop runtime/i.test(message);
    const isAllowedProfile = (message) => allowedProfilePatterns.some((pattern) => pattern.test(message));
    return {
      messages,
      runtime: messages.filter(isRuntime),
      allowedFixtureProfile: messages.filter((message) => isAllowedProfile(message)),
      unexpected: messages.filter((message) => !isRuntime(message) && !isAllowedProfile(message)),
      model: messages.filter((message) => /construct|structural path|editable model|active model/i.test(message)),
    };
  };
  const inspectDialogBounds = () => dialog.evaluate((node) => {
    const rect = node.getBoundingClientRect();
    return {
      viewport: { width: window.innerWidth, height: window.innerHeight },
      rect: { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height },
      withinHorizontalViewport: rect.left >= -2 && rect.right <= window.innerWidth + 2,
      verticalScrollAvailable: node.scrollHeight > node.clientHeight + 2,
      pageHorizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth)
        > document.documentElement.clientWidth + 2,
    };
  });

  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadOlsFixture === "function");
    if (!check.fixtureApiPresent) {
      recordFailure("regression-bootstrap-browser-fixture-api", `The genuine data-only regression smoke fixture was not exposed at ${viewport.id}.`, check);
      return;
    }
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadOlsFixture());
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dataSurface = await page.locator('.nd-app[data-surface="data"]').count() === 1;
    check.visibleModelNodes = await page.locator(".react-flow__node-latent").count();

    trigger = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
    check.analyzeCommandCount = await trigger.count();
    if (check.analyzeCommandCount !== 1) {
      recordFailure("regression-bootstrap-analyze-command", `The Data toolbar did not expose exactly one Analyze command at ${viewport.id}.`, check);
      return;
    }
    await trigger.focus();
    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.dialogOpened = await dialog.isVisible().catch(() => false);
    if (!check.dialogOpened) {
      recordFailure("regression-bootstrap-dialog-open", `Analyze did not open the calculation dialog at ${viewport.id}.`, check);
      return;
    }

    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    check.catalogCount = await listbox.getByRole("option").count();
    const selection = await selectCalculationMethod(dialog, "regression");
    check.selectedMethod = compactCalculationText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent().catch(() => ""));
    check.linkage = selection.linkage;
    check.category = compactCalculationText(await dialog.locator("#nd-calculation-category-standalone").textContent().catch(() => ""));

    const regressionType = dialog.locator("#nd-calculation-regression-type");
    const outcome = dialog.locator("#nd-calculation-regression-outcome");
    const bootstrapToggle = dialog.locator("#nd-calculation-regression-bootstrap");
    await regressionType.selectOption("ols");
    await outcome.selectOption("outcome");
    const roleFieldsets = dialog.locator(".nd-ols-settings fieldset.nd-pca-variables");
    const predictorInput = roleFieldsets.nth(0).locator("label").filter({ hasText: /^predictor$/ }).locator('input[type="checkbox"]');
    const controlInput = roleFieldsets.nth(1).locator("label").filter({ hasText: /^control$/ }).locator('input[type="checkbox"]');
    if (!await predictorInput.isChecked()) await predictorInput.check();
    if (!await controlInput.isChecked()) await controlInput.check();
    check.regressionTypeOptions = await regressionType.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    })));
    check.outcome = await outcome.inputValue();
    check.roles = await inspectRoles();

    await bootstrapToggle.selectOption("off");
    await bootstrapToggle.focus();
    await page.keyboard.press("ArrowDown");
    await page.waitForFunction(() => document.querySelector("#nd-calculation-regression-bootstrap")?.value === "enabled", null, { timeout: 1_000 }).catch(() => null);
    const samples = dialog.locator("#nd-calculation-regression-bootstrap-samples");
    const workers = dialog.locator("#nd-calculation-regression-bootstrap-workers");
    const seed = dialog.locator("#nd-calculation-seed");
    await samples.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    check.bootstrap = {
      value: await bootstrapToggle.inputValue(),
      options: await bootstrapToggle.locator("option").evaluateAll((options) => options.map((option) => ({
        value: option.value,
        label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
      }))),
      samples: await inspectNumberInput(samples),
      workers: await inspectNumberInput(workers),
      seed: await inspectNumberInput(seed),
      scope: compactCalculationText(await dialog.locator("#nd-calculation-regression-bootstrap-scope strong").textContent().catch(() => "")),
      toggleFocused: await bootstrapToggle.evaluate((node) => document.activeElement === node),
    };
    check.accessibility = {
      labeledRegressionType: await explicitLabelAssociationCount(regressionType),
      labeledOutcome: await explicitLabelAssociationCount(outcome),
      labeledBootstrapToggle: await explicitLabelAssociationCount(bootstrapToggle),
      labeledSamples: await dialog.getByLabel("Bootstrap samples", { exact: true }).count(),
      labeledWorkers: await dialog.getByLabel("Parallel workers", { exact: true }).count(),
      labeledSeed: await dialog.getByLabel("Seed", { exact: true }).count(),
      predictorGroup: await dialog.getByRole("group", { name: /^Predictors \(1 selected\)$/ }).count(),
      controlGroup: await dialog.getByRole("group", { name: /^Controls \(1 selected, optional\)$/ }).count(),
      distinctControlIds: await dialog.locator([
        "#nd-calculation-regression-type",
        "#nd-calculation-regression-outcome",
        "#nd-calculation-regression-bootstrap",
        "#nd-calculation-regression-bootstrap-samples",
        "#nd-calculation-regression-bootstrap-workers",
        "#nd-calculation-seed",
      ].join(", ")).count(),
    };

    const olsStart = dialog.getByRole("button", { name: "Start OLS regression with bootstrap", exact: true });
    check.ols = {
      type: await regressionType.inputValue(),
      outcome: await outcome.inputValue(),
      roles: await inspectRoles(),
      calculationBasis: await readSettingNote("Calculation basis"),
      variableData: await readSettingNote("Variable data"),
      uncertainty: await readSettingNote("Uncertainty"),
      validatedScope: await readSettingNote("Validated scope"),
      blockers: await inspectBlockers(),
      startCommandCount: await olsStart.count(),
      startCommandDisabled: await olsStart.isDisabled().catch(() => false),
      truthAndOverflow: await inspectCalculationTruthAndOverflow(dialog),
      dialogBounds: await inspectDialogBounds(),
      noPhantomResult: await page.locator(".nd-result-tree, .nd-result-table").count() === 0,
    };
    await capture(page, "regression-bootstrap-ols-dialog", olsSequence, viewport, { dialog: "calculation" });

    await regressionType.selectOption("logistic");
    await regressionType.focus();
    const logisticProfile = dialog.locator("#nd-calculation-logistic-profile");
    await logisticProfile.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
    await page.waitForFunction(() => document.querySelector("#nd-calculation-logistic-profile")?.getAttribute("aria-busy") === "false", null, { timeout: 1_000 }).catch(() => null);
    const logisticStart = dialog.getByRole("button", { name: "Start binary logistic regression with bootstrap", exact: true });
    check.logistic = {
      type: await regressionType.inputValue(),
      typeFocused: await regressionType.evaluate((node) => document.activeElement === node),
      outcome: await outcome.inputValue(),
      roles: await inspectRoles(),
      bootstrapValue: await bootstrapToggle.inputValue(),
      samples: await samples.inputValue(),
      workers: await workers.inputValue(),
      seed: await seed.inputValue(),
      calculationBasis: await readSettingNote("Calculation basis"),
      variableData: await readSettingNote("Variable data"),
      uncertainty: await readSettingNote("Uncertainty"),
      validatedScope: await readSettingNote("Validated scope"),
      bootstrapScope: compactCalculationText(await dialog.locator("#nd-calculation-regression-bootstrap-scope strong").textContent().catch(() => "")),
      profile: {
        role: await logisticProfile.getAttribute("role"),
        ariaLive: await logisticProfile.getAttribute("aria-live"),
        ariaBusy: await logisticProfile.getAttribute("aria-busy"),
        text: compactCalculationText(await logisticProfile.textContent().catch(() => "")),
      },
      blockers: await inspectBlockers([/36 non-missing outcome rows are not coded exactly 0 or 1/i, /must contain both class 0 and class 1/i]),
      startCommandCount: await logisticStart.count(),
      startCommandDisabled: await logisticStart.isDisabled().catch(() => false),
      truthAndOverflow: await inspectCalculationTruthAndOverflow(dialog),
      dialogBounds: await inspectDialogBounds(),
      noPhantomResult: await page.locator(".nd-result-tree, .nd-result-table").count() === 0,
    };
    await capture(page, "regression-bootstrap-logistic-dialog", logisticSequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);

    const completedLoaderName = await page.evaluate(() => Object.keys(window.__QUICKPLS_SMOKE__ ?? {}).find((name) => (
      /load/i.test(name) && /regression/i.test(name) && /bootstrap/i.test(name) && /(completed|result)/i.test(name)
    )) ?? null);
    check.completedResult = {
      genuineSmokeLoader: completedLoaderName,
      inspected: false,
      synthesizedByHarness: false,
      runOptionCount: 0,
      groupCount: 0,
      tableCount: 0,
    };
    if (completedLoaderName) {
      await page.evaluate(async (name) => {
        const loader = window.__QUICKPLS_SMOKE__?.[name];
        if (typeof loader === "function") await loader();
      }, completedLoaderName);
      await setSurface(page, "results");
      await page.locator(".nd-result-tree").waitFor({ state: "visible", timeout: 5_000 }).catch(() => null);
      check.completedResult.runOptionCount = await page.locator(".nd-run-select select option").count();
      check.completedResult.groupCount = await page.getByRole("treeitem", { name: /^(OLS regression|Binary logistic regression) with bootstrap$/ }).count();
      check.completedResult.tableCount = await page.getByRole("treeitem", { name: /^(Regression bootstrap summary|Bootstrap coefficient inference|Percentile confidence intervals \(primary\)|BCa confidence intervals \(conditional\)|Bootstrap odds-ratio intervals)$/ }).count();
      check.completedResult.inspected = check.completedResult.runOptionCount > 0
        && check.completedResult.groupCount === 1
        && check.completedResult.tableCount >= 4;
    } else {
      recordSkip("regression-bootstrap-completed-results-browser", "The smoke runtime provides genuine regression setup data but no completed regression-bootstrap run fixture. The browser harness did not synthesize a run, coefficient, interval, failure, or odds-ratio row.", {
        viewport: viewport.id,
        featureId: "qpls3.standalone.regression_bootstrap",
        methodVersion: "regression_bootstrap_v1",
        requiredPackagedFollowUp: "Run genuine OLS and binary-logistic case-resampling jobs through packaged Tauri; inspect summary/coefficient/percentile/conditional-BCa tables, conditional failures, logistic odds ratios, XLSX, save, archive witness, and same-run reopen.",
      });
    }
  } finally {
    if (await dialog.isVisible().catch(() => false)) await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    evidence.checks.regressionBootstrap.push(check);
  }

  const expectedRegressionTypes = [
    { value: "ols", label: "Ordinary least squares" },
    { value: "logistic", label: "Binary logistic (outcome coded 0/1)" },
    { value: "process", label: "Graph-defined Path Analysis / PROCESS" },
  ];
  const expectedBootstrapOptions = [
    { value: "off", label: "Off" },
    { value: "enabled", label: "Case-resampling bootstrap" },
  ];
  const expectedContinuousFixtureProfileBlockers = [
    "36 non-missing outcome rows are not coded exactly 0 or 1; The listwise-complete outcome must contain both class 0 and class 1.",
    "36 non-missing outcome rows are not coded exactly 0 or 1",
    "The listwise-complete outcome must contain both class 0 and class 1",
  ];
  if (JSON.stringify(check.fixture) !== JSON.stringify({ variables: 5, models: 0 })
    || !check.dataSurface || check.visibleModelNodes !== 0 || check.analyzeCommandCount !== 1) {
    recordFailure("regression-bootstrap-data-only-fixture", `Regression bootstrap did not begin from the genuine five-variable, zero-model smoke workspace at ${viewport.id}.`, check);
  }
  if (!check.dialogOpened || check.catalogCount !== nativeCalculationMethods.length || check.selectedMethod !== "Regression"
    || !check.linkage?.linkage || check.category !== "Standalone analysis"
    || JSON.stringify(check.regressionTypeOptions) !== JSON.stringify(expectedRegressionTypes)) {
    recordFailure("regression-bootstrap-method-toggle", `Regression bootstrap did not expose the linked Regression catalog entry with exact OLS/logistic choices at ${viewport.id}.`, check);
  }
  if (check.outcome !== "outcome" || JSON.stringify(check.roles?.selectedPredictors) !== JSON.stringify(["predictor"])
    || JSON.stringify(check.roles?.selectedControls) !== JSON.stringify(["control"])
    || check.roles?.fieldsetCount !== 2 || check.roles?.predictors?.legend !== "Predictors (1 selected)"
    || check.roles?.controls?.legend !== "Controls (1 selected, optional)") {
    recordFailure("regression-bootstrap-variable-roles", `Regression bootstrap did not preserve the exact outcome, predictor, and optional-control role contract at ${viewport.id}.`, check);
  }
  if (check.bootstrap?.value !== "enabled" || JSON.stringify(check.bootstrap?.options) !== JSON.stringify(expectedBootstrapOptions)
    || JSON.stringify(check.bootstrap?.samples) !== JSON.stringify({ count: 1, value: "10000", min: "99", max: "10000", step: "1" })
    || check.bootstrap?.workers?.count !== 1 || check.bootstrap?.workers?.value !== "1"
    || check.bootstrap?.workers?.min !== "1" || check.bootstrap?.workers?.max !== "64"
    || check.bootstrap?.seed?.count !== 1 || check.bootstrap?.seed?.value !== "20260718"
    || check.bootstrap?.seed?.min !== "0" || check.bootstrap?.seed?.max !== "4294967295"
    || check.bootstrap?.scope !== nativeRegressionBootstrapScopeNote || !check.bootstrap?.toggleFocused) {
    recordFailure("regression-bootstrap-resampling-controls", `Regression bootstrap did not expose enabled 10,000-sample case resampling with exact worker, seed, range, focus, and scope contracts at ${viewport.id}.`, check);
  }
  if (!check.accessibility || Object.values(check.accessibility).some((count) => count !== 1 && count !== 6)
    || check.accessibility?.distinctControlIds !== 6) {
    recordFailure("regression-bootstrap-control-accessibility", `Regression bootstrap controls or variable groups were not uniquely labelled and focusable at ${viewport.id}.`, check);
  }
  if (check.ols?.type !== "ols" || check.ols?.outcome !== "outcome"
    || check.ols?.calculationBasis !== "Raw-value OLS with intercept (fixed)"
    || check.ols?.variableData !== "Unstandardized numeric values (fixed)"
    || check.ols?.uncertainty !== "HC3 robust SE; two-sided 95% CI (fixed)"
    || check.ols?.validatedScope !== nativeOlsScopeNote
    || check.ols?.blockers?.runtime?.length !== 1 || check.ols?.blockers?.unexpected?.length !== 0
    || check.ols?.blockers?.model?.length !== 0 || check.ols?.startCommandCount !== 1 || !check.ols?.startCommandDisabled) {
    recordFailure("regression-bootstrap-ols-setup", `The OLS bootstrap state exposed a setup blocker, stale disclosure, or incorrect start contract at ${viewport.id}.`, check);
  }
  if (check.logistic?.type !== "logistic" || !check.logistic?.typeFocused || check.logistic?.outcome !== "outcome"
    || check.logistic?.bootstrapValue !== "enabled" || check.logistic?.samples !== "10000"
    || check.logistic?.workers !== "1" || check.logistic?.seed !== "20260718"
    || check.logistic?.calculationBasis !== "Binary logistic maximum likelihood with intercept (fixed)"
    || check.logistic?.variableData !== "Unstandardized numeric values (fixed)"
    || check.logistic?.uncertainty !== "Maximum-likelihood SE; Wald z and two-sided 95% CI; odds ratios (fixed)"
    || check.logistic?.validatedScope !== nativeLogisticScopeNote || check.logistic?.bootstrapScope !== nativeRegressionBootstrapScopeNote
    || check.logistic?.profile?.role !== "status" || check.logistic?.profile?.ariaLive !== "polite" || check.logistic?.profile?.ariaBusy !== "false"
    || !check.logistic?.profile?.text.includes("36 complete cases: 0 class 0 and 0 class 1; 0 omitted by listwise deletion")
    || check.logistic?.blockers?.runtime?.length !== 1
    || JSON.stringify(check.logistic?.blockers?.allowedFixtureProfile) !== JSON.stringify(expectedContinuousFixtureProfileBlockers)
    || check.logistic?.blockers?.unexpected?.length !== 0 || check.logistic?.blockers?.model?.length !== 0
    || check.logistic?.startCommandCount !== 1 || !check.logistic?.startCommandDisabled) {
    recordFailure("regression-bootstrap-logistic-toggle", `The genuine continuous smoke fixture was not truthfully toggled to logistic bootstrap with only its exact 0/1 profile limitations at ${viewport.id}.`, check);
  }
  if (!check.ols?.truthAndOverflow?.noFabricatedRunState || !check.ols?.truthAndOverflow?.noHorizontalOverflow
    || !check.logistic?.truthAndOverflow?.noFabricatedRunState || !check.logistic?.truthAndOverflow?.noHorizontalOverflow
    || !check.ols?.dialogBounds?.withinHorizontalViewport || check.ols?.dialogBounds?.pageHorizontalOverflow
    || !check.logistic?.dialogBounds?.withinHorizontalViewport || check.logistic?.dialogBounds?.pageHorizontalOverflow
    || !check.ols?.noPhantomResult || !check.logistic?.noPhantomResult
    || !check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("regression-bootstrap-browser-truth-layout", `Regression bootstrap exposed fabricated results, responsive overflow, or broken close focus at ${viewport.id}.`, check);
  }
  if (check.completedResult?.genuineSmokeLoader && !check.completedResult?.inspected) {
    recordFailure("regression-bootstrap-genuine-result-inspection", `A genuine completed regression-bootstrap smoke loader was present but its result navigation was incomplete at ${viewport.id}.`, check.completedResult);
  }
}

async function auditProcessV2DialogFromImportedProject(page, viewport, sequence) {
  const check = {
    viewport: viewport.id,
    fixtureApiPresent: false,
    fixture: null,
    fixtureImport: { mechanism: "query-gated production smoke API", loader: "loadProcessV2Fixture" },
    dataSurface: false,
    dialogOpened: false,
    regressionTypeOptions: [],
    regressionType: "",
    setup: null,
    accessibility: null,
    truthAndOverflow: null,
    dialogBounds: null,
    completedResult: {
      available: false,
      synthesizedByHarness: false,
      packagedFollowUp: "Run, cancel, verify accessible non-color conditional/JN plots plus their complete point tables, export, save, inspect the archive witness, and reopen the genuine 10,000-resample PROCESS v2 result in packaged Tauri.",
    },
  };
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  let trigger = null;
  const desiredPaths = [
    ["X", "Y"], ["X", "M1"], ["M1", "M2"], ["M2", "Y"],
    ["X", "M3"], ["M3", "Y"], ["X", "M4"], ["M4", "Y"],
  ];

  const waitForCount = async (selector, count) => {
    await page.waitForFunction(({ selector: value, count: expected }) => (
      document.querySelectorAll(value).length === expected
    ), { selector, count }, { timeout: 2_000 });
  };
  const setPath = async (index, from, to) => {
    await dialog.locator(`#nd-process-path-to-${index}`).selectOption(to);
    await dialog.locator(`#nd-process-path-from-${index}`).selectOption(from);
    await page.waitForFunction(({ index: row, from: source, to: target }) => (
      document.querySelector(`#nd-process-path-from-${row}`)?.value === source
      && document.querySelector(`#nd-process-path-to-${row}`)?.value === target
    ), { index, from, to }, { timeout: 2_000 });
  };
  const inspectFocusable = async (selector) => {
    const control = dialog.locator(selector);
    const count = await control.count();
    const disabled = count === 1 ? await control.isDisabled().catch(() => true) : null;
    const visible = count === 1 ? await control.isVisible().catch(() => false) : false;
    if (count !== 1 || disabled || !visible) {
      return { selector, count, disabled, visible, focused: false };
    }
    await control.focus();
    return {
      selector,
      count,
      disabled,
      visible,
      focused: await control.evaluate((node) => document.activeElement === node),
    };
  };
  const exerciseStableRowSelects = async (rowSelector) => {
    const rowOrder = async () => dialog.locator(rowSelector).evaluateAll((rows) => rows.map((row) => (
      Array.from(row.querySelectorAll("select")).map((select) => select.id)
    )));
    const initialRowOrder = await rowOrder();
    const selectIds = initialRowOrder.flat();
    const mutations = [];
    for (const id of selectIds) {
      const select = dialog.locator(`#${id}`);
      const state = await select.evaluate((node) => ({
        value: node.value,
        disabled: node.disabled,
        alternative: Array.from(node.options).find((option) => !option.disabled && option.value !== node.value)?.value ?? null,
      }));
      if (state.disabled) {
        mutations.push({ id, disabled: true, changed: false, focusedBefore: false, focusedAfterChange: false, rowOrderUnchanged: true });
        continue;
      }
      await select.focus();
      const focusedBefore = await select.evaluate((node) => document.activeElement === node);
      if (state.alternative === null) {
        mutations.push({ id, disabled: false, changed: false, focusedBefore, focusedAfterChange: false, rowOrderUnchanged: false });
        continue;
      }
      await select.selectOption(state.alternative);
      const focusedAfterChange = await page.waitForFunction(({ id: controlId, value }) => {
        const node = document.getElementById(controlId);
        return node instanceof HTMLSelectElement
          && node.value === value
          && document.activeElement?.id === controlId;
      }, { id, value: state.alternative }, { timeout: 1_000 }).then(() => true).catch(() => false);
      const afterChangeOrder = await rowOrder();
      const rowOrderUnchanged = JSON.stringify(afterChangeOrder) === JSON.stringify(initialRowOrder);
      await dialog.locator(`#${id}`).selectOption(state.value).catch(() => null);
      mutations.push({
        id, disabled: false, changed: true, focusedBefore, focusedAfterChange,
        rowOrderUnchanged,
      });
    }
    const finalRowOrder = await rowOrder();
    const enabledMutations = mutations.filter((row) => !row.disabled);
    return {
      initialRowOrder,
      finalRowOrder,
      rowCount: initialRowOrder.length,
      selectCount: selectIds.length,
      enabledSelectCount: enabledMutations.length,
      disabledSelectCount: mutations.length - enabledMutations.length,
      changedSelectCount: enabledMutations.filter((row) => row.changed).length,
      mutations,
      passed: selectIds.length > 0
        && enabledMutations.length > 0
        && enabledMutations.every((row) => row.changed && row.focusedBefore && row.focusedAfterChange && row.rowOrderUnchanged)
        && JSON.stringify(finalRowOrder) === JSON.stringify(initialRowOrder),
    };
  };

  try {
    check.fixtureApiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadProcessV2Fixture === "function");
    if (!check.fixtureApiPresent) throw new Error("loadProcessV2Fixture is not exposed by the production smoke API.");
    check.fixture = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadProcessV2Fixture());
    await page.locator('.nd-app[data-surface="data"]').waitFor({ state: "visible", timeout: 2_000 });
    await page.getByLabel("Project data navigator").getByRole("button", { name: "X", exact: true })
      .waitFor({ state: "visible", timeout: 2_000 });
    check.dataSurface = await page.locator('.nd-app[data-surface="data"]').count() === 1;
    trigger = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
    if (await trigger.count() !== 1) throw new Error(`PROCESS v2 requires exactly one model-free Analyze command at ${viewport.id}.`);
    await trigger.focus();
    await trigger.click();
    await dialog.waitFor({ state: "visible", timeout: 2_000 });
    check.dialogOpened = true;
    await selectCalculationMethod(dialog, "regression");

    const regressionType = dialog.locator("#nd-calculation-regression-type");
    check.regressionTypeOptions = await regressionType.locator("option").evaluateAll((options) => options.map((option) => ({
      value: option.value,
      label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    })));
    await regressionType.selectOption("process");
    check.regressionType = await regressionType.inputValue();
    await dialog.locator("#nd-calculation-process-graph").waitFor({ state: "visible", timeout: 2_000 });
    await dialog.locator("#nd-process-outcome").selectOption("Y");
    await dialog.locator("#nd-process-focal").selectOption("X");
    await waitForCount("[data-process-path-row]", 1);
    await setPath(0, "X", "Y");
    for (let index = 1; index < desiredPaths.length; index += 1) {
      await dialog.locator("#nd-process-add-path").click();
      await waitForCount("[data-process-path-row]", index + 1);
      await setPath(index, desiredPaths[index][0], desiredPaths[index][1]);
    }
    const stablePathRows = await exerciseStableRowSelects("[data-process-path-row]");
    for (let index = 0; index < desiredPaths.length; index += 1) {
      await setPath(index, desiredPaths[index][0], desiredPaths[index][1]);
    }

    for (let index = 0; index < 2; index += 1) {
      await dialog.locator("#nd-process-add-moderator").click();
      await waitForCount("[data-process-moderator-row]", index + 1);
    }
    await dialog.locator("#nd-process-moderator-variable-0").selectOption("W");
    await dialog.locator("#nd-process-moderator-scale-0").selectOption("continuous");
    await dialog.locator("#nd-process-moderator-variable-1").selectOption("B");
    await dialog.locator("#nd-process-moderator-scale-1").selectOption("binary_0_1");
    const stableModeratorRows = await exerciseStableRowSelects("[data-process-moderator-row]");
    await dialog.locator("#nd-process-moderator-variable-0").selectOption("W");
    await dialog.locator("#nd-process-moderator-scale-0").selectOption("continuous");
    await dialog.locator("#nd-process-moderator-variable-1").selectOption("B");
    await dialog.locator("#nd-process-moderator-scale-1").selectOption("binary_0_1");

    const desiredModerations = [
      { edge: "X -> Y", primary: "W", conditioning: "B" },
      { edge: "X -> M3", primary: "W", conditioning: "" },
      { edge: "M4 -> Y", primary: "B", conditioning: "" },
    ];
    for (let index = 0; index < desiredModerations.length; index += 1) {
      await dialog.locator("#nd-process-add-moderation").click();
      await waitForCount("[data-process-moderation-row]", index + 1);
      const expected = desiredModerations[index];
      await dialog.locator(`#nd-process-moderation-edge-${index}`).selectOption({ label: expected.edge });
      await dialog.locator(`#nd-process-moderation-primary-${index}`).selectOption(expected.primary);
      const conditioning = dialog.locator(`#nd-process-moderation-conditioning-${index}`);
      if (!await conditioning.isDisabled()) await conditioning.selectOption(expected.conditioning);
    }
    const stableModerationRows = await exerciseStableRowSelects("[data-process-moderation-row]");
    for (let index = 0; index < desiredModerations.length; index += 1) {
      const expected = desiredModerations[index];
      await dialog.locator(`#nd-process-moderation-edge-${index}`).selectOption({ label: expected.edge });
      await dialog.locator(`#nd-process-moderation-primary-${index}`).selectOption(expected.primary);
      const conditioning = dialog.locator(`#nd-process-moderation-conditioning-${index}`);
      if (!await conditioning.isDisabled()) await conditioning.selectOption(expected.conditioning);
    }

    const control = dialog.locator(".nd-process-controls label").filter({ hasText: /^C$/ }).locator('[data-process-control]');
    if (await control.count() !== 1) throw new Error("PROCESS v2 fixture did not expose C as one eligible control.");
    await control.check();

    const bootstrap = dialog.locator("#nd-calculation-regression-bootstrap");
    await bootstrap.selectOption("enabled");
    const samples = dialog.locator("#nd-calculation-regression-bootstrap-samples");
    const workers = dialog.locator("#nd-calculation-regression-bootstrap-workers");
    const seed = dialog.locator("#nd-calculation-seed");
    await samples.waitFor({ state: "visible", timeout: 2_000 });
    await page.waitForFunction(() => (
      document.querySelector("#nd-calculation-process-profile")?.textContent?.includes("5 OLS equations verified")
    ), null, { timeout: 2_000 }).catch(() => null);

    const paths = await dialog.locator("[data-process-path-row]").evaluateAll((rows) => rows.map((row) => ({
      from: row.querySelector("select[id^='nd-process-path-from-']")?.value ?? "",
      to: row.querySelector("select[id^='nd-process-path-to-']")?.value ?? "",
    })));
    const moderators = await dialog.locator("[data-process-moderator-row]").evaluateAll((rows) => rows.map((row) => ({
      variable: row.querySelector("select[id^='nd-process-moderator-variable-']")?.value ?? "",
      scale: row.querySelector("select[id^='nd-process-moderator-scale-']")?.value ?? "",
    })));
    const moderations = await dialog.locator("[data-process-moderation-row]").evaluateAll((rows) => rows.map((row) => ({
      edge: row.querySelector("select[id^='nd-process-moderation-edge-']")?.selectedOptions[0]?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      primary: row.querySelector("select[id^='nd-process-moderation-primary-']")?.value ?? "",
      conditioning: row.querySelector("select[id^='nd-process-moderation-conditioning-']")?.value ?? "",
    })));
    const scope = compactCalculationText(await dialog.locator("#nd-calculation-process-scope strong").textContent().catch(() => ""));
    const expectedScope = compactCalculationText(`${nativeProcessV2ScopeNote} ${nativeProcessV2ProbeDisclosure}`);
    const bootstrapScope = compactCalculationText(await dialog.locator("#nd-calculation-regression-bootstrap-scope strong").textContent().catch(() => ""));
    const profile = dialog.locator("#nd-calculation-process-profile");
    const profileText = compactCalculationText(await profile.locator("strong").textContent().catch(() => ""));
    const previewTitle = compactCalculationText(await dialog.locator("#nd-process-preview-title").textContent().catch(() => ""));
    const previewDescription = compactCalculationText(await dialog.locator("#nd-process-preview-description").textContent().catch(() => ""));
    const previewSvg = dialog.locator('#nd-process-graph-preview svg[role="img"]');
    const previewSvgCount = await previewSvg.count();
    const previewAccessibility = previewSvgCount === 1
      ? await previewSvg.evaluate((svg) => {
        const ids = svg.getAttribute("aria-labelledby")?.split(/\s+/).filter(Boolean) ?? [];
        const dialogNode = svg.closest('.nd-dialog-calculation[role="dialog"]');
        const references = ids.map((id) => {
          const node = document.getElementById(id);
          return {
            id,
            exists: Boolean(node),
            withinDialog: Boolean(node && dialogNode?.contains(node)),
            tagName: node?.tagName.toLowerCase() ?? null,
            text: node?.textContent?.replace(/\s+/g, " ").trim() ?? "",
          };
        });
        return {
          count: 1,
          role: svg.getAttribute("role"),
          ariaLabelledBy: svg.getAttribute("aria-labelledby"),
          ids,
          references,
          passed: svg.getAttribute("role") === "img"
            && ids.length === 2
            && references.every((reference) => reference.exists && reference.withinDialog)
            && references[0]?.tagName === "title"
            && references[1]?.tagName === "desc",
        };
      })
      : {
        count: previewSvgCount,
        role: null,
        ariaLabelledBy: null,
        ids: [],
        references: [],
        passed: false,
      };
    const previewAccessible = previewAccessibility.passed;
    const expectedPreview = "X -> Y moderated by W and B; X -> M1; M1 -> M2; M2 -> Y; X -> M3 moderated by W; M3 -> Y; X -> M4; M4 -> Y moderated by B";
    const blockerMessages = (await dialog.locator(".nd-blocker li").allTextContents()).map(compactCalculationText);
    const runtimeBlockers = blockerMessages.filter((message) => /offline QuickPLS desktop runtime/i.test(message));
    const unexpectedBlockers = blockerMessages.filter((message) => !/offline QuickPLS desktop runtime/i.test(message));
    const selectedControls = await dialog.locator('[data-process-control]:checked').evaluateAll((inputs) => (
      inputs.map((input) => input.closest("label")?.querySelector("span")?.textContent?.trim() ?? "")
    ));
    const pathLegend = compactCalculationText(await dialog.locator("#nd-calculation-process-graph fieldset").nth(0).locator("legend").textContent());
    const controlLegend = compactCalculationText(await dialog.locator(".nd-process-controls legend").textContent());
    const predictorCapacity = pathLegend.match(/(\d+)\/(\d+) graph predictors/i);
    const controlCapacity = controlLegend.match(/Controls \((\d+)\/(\d+)/i);
    const equationTermCapacity = scope.match(/the (\d+)-term ceiling/i);
    const startButton = dialog.locator("footer button.primary");
    const labels = [
      "#nd-calculation-regression-type", "#nd-process-outcome", "#nd-process-focal",
      ...desiredPaths.flatMap((_, index) => [`#nd-process-path-from-${index}`, `#nd-process-path-to-${index}`]),
      "#nd-process-moderator-variable-0", "#nd-process-moderator-scale-0",
      "#nd-process-moderator-variable-1", "#nd-process-moderator-scale-1",
      ...desiredModerations.flatMap((_, index) => [
        `#nd-process-moderation-edge-${index}`, `#nd-process-moderation-primary-${index}`,
        `#nd-process-moderation-conditioning-${index}`,
      ]),
      "#nd-calculation-regression-bootstrap", "#nd-calculation-regression-bootstrap-samples",
      "#nd-calculation-regression-bootstrap-workers", "#nd-calculation-seed",
    ];
    const controlsLabeled = await dialog.evaluate((dialogNode, selectors) => selectors.every((selector) => {
      const node = dialogNode.querySelector(selector);
      return node instanceof HTMLInputElement || node instanceof HTMLSelectElement
        ? Array.from(node.labels ?? []).some((label) => label.htmlFor === node.id)
        : false;
    }), labels);
    const groupsNamed = await dialog.locator("#nd-calculation-process-graph fieldset").evaluateAll((fieldsets) => (
      fieldsets.length === 4 && fieldsets.every((fieldset) => Boolean(fieldset.querySelector("legend")?.textContent?.trim()))
    ));
    const focusSelectors = [
      "#nd-calculation-regression-type", "#nd-process-outcome",
      "#nd-process-add-path", "#nd-process-moderator-variable-0",
      "#nd-calculation-regression-bootstrap", "#nd-calculation-regression-bootstrap-samples",
      "#nd-calculation-regression-bootstrap-workers", "#nd-calculation-seed",
    ];
    const focusChecks = [];
    for (const selector of focusSelectors) focusChecks.push(await inspectFocusable(selector));
    const keyboardReachable = focusChecks.every((row) => row.focused);

    check.setup = {
      outcome: await dialog.locator("#nd-process-outcome").inputValue(),
      focal: await dialog.locator("#nd-process-focal").inputValue(),
      pathRows: paths.length,
      paths,
      pathsExact: JSON.stringify(paths) === JSON.stringify(desiredPaths.map(([from, to]) => ({ from, to }))),
      moderatorRows: moderators.length,
      moderators,
      moderatorsExact: JSON.stringify(moderators) === JSON.stringify([
        { variable: "W", scale: "continuous" }, { variable: "B", scale: "binary_0_1" },
      ]),
      moderationRows: moderations.length,
      moderations,
      moderationsExact: JSON.stringify(moderations) === JSON.stringify(desiredModerations),
      stableRowIdentity: {
        paths: stablePathRows,
        moderators: stableModeratorRows,
        moderations: stableModerationRows,
        passed: stablePathRows.passed && stableModeratorRows.passed && stableModerationRows.passed,
      },
      selectedControls,
      capacity: {
        topLevelPredictors: predictorCapacity ? Number(predictorCapacity[1]) : null,
        topLevelPredictorsMaximum: predictorCapacity ? Number(predictorCapacity[2]) : null,
        controls: controlCapacity ? Number(controlCapacity[1]) : null,
        controlsMaximum: controlCapacity ? Number(controlCapacity[2]) : null,
        equationNonInterceptTermsMaximum: equationTermCapacity ? Number(equationTermCapacity[1]) : null,
      },
      bootstrap: await bootstrap.inputValue(),
      samples: await samples.inputValue(),
      samplesBounds: { min: await samples.getAttribute("min"), max: await samples.getAttribute("max"), step: await samples.getAttribute("step") },
      workers: await workers.inputValue(),
      workersBounds: { min: await workers.getAttribute("min"), max: await workers.getAttribute("max"), step: await workers.getAttribute("step") },
      seed: await seed.inputValue(),
      seedBounds: { min: await seed.getAttribute("min"), max: await seed.getAttribute("max"), step: await seed.getAttribute("step") },
      startLabel: compactCalculationText(await startButton.textContent().catch(() => "")),
      startDisabledInBrowserPreview: await startButton.isDisabled(),
      runtimeBlockers,
      unexpectedBlockers,
      profileReady: await profile.getAttribute("aria-busy") === "false" && profileText === "62 global listwise-complete cases; 2 rows omitted; 5 OLS equations verified",
      profileText,
      scopeExact: scope === expectedScope,
      scope,
      bootstrapScopeExact: bootstrapScope === nativeProcessV2BootstrapScopeNote,
      bootstrapScope,
      previewExact: previewTitle === "Graph-defined path analysis preview" && previewDescription === expectedPreview,
      previewAccessible,
      preview: { title: previewTitle, description: previewDescription, expectedDescription: expectedPreview },
      previewAccessibility,
      focusChecks,
    };
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    check.dialogBounds = await dialog.evaluate((node) => {
      const rect = node.getBoundingClientRect();
      return {
        withinHorizontalViewport: rect.left >= -2 && rect.right <= window.innerWidth + 2,
        pageHorizontalOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) > document.documentElement.clientWidth + 2,
      };
    });
    check.completedResult.available = await page.locator(".nd-result-tree").count() > 0;
    await capture(page, "process-v2-dialog", sequence, viewport, { dialog: "calculation" });
    const closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
    check.accessibility = {
      controlsLabeled,
      groupsNamed,
      keyboardReachable,
      focusRestored: closeFocus.dialogClosed && closeFocus.focusRestored,
    };
    recordSkip("process-v2-completed-results-browser", "The imported project provides genuine PROCESS setup data but no completed PROCESS v2 run. The visual harness did not synthesize a run, equation, effect, interval, plot, failure, or Johnson-Neyman row.", {
      viewport: viewport.id,
      requiredPackagedFollowUp: check.completedResult.packagedFollowUp,
    });
  } catch (error) {
    recordFailure("process-v2-visual-harness", `PROCESS v2 setup acceptance failed at ${viewport.id}: ${String(error?.message ?? error)}`, check);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      check.accessibility = {
        ...(check.accessibility ?? { controlsLabeled: false, groupsNamed: false, keyboardReachable: false }),
        focusRestored: (await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => ({ focusRestored: false }))).focusRestored,
      };
    }
    evidence.checks.processV2.push(check);
  }

  const expectedTypeOptions = [
    { value: "ols", label: "Ordinary least squares" },
    { value: "logistic", label: "Binary logistic (outcome coded 0/1)" },
    { value: "process", label: "Graph-defined Path Analysis / PROCESS" },
  ];
  if (!check.fixtureApiPresent || JSON.stringify(check.fixture) !== JSON.stringify({ variables: 9, models: 0 })
    || !check.dataSurface || !check.dialogOpened || check.regressionType !== "process"
    || JSON.stringify(check.regressionTypeOptions) !== JSON.stringify(expectedTypeOptions)
    || !check.setup?.pathsExact || !check.setup?.moderatorsExact || !check.setup?.moderationsExact
    || !check.setup?.stableRowIdentity?.passed
    || JSON.stringify(check.setup?.selectedControls) !== JSON.stringify(["C"])
    || JSON.stringify(check.setup?.capacity) !== JSON.stringify({
      topLevelPredictors: 7, topLevelPredictorsMaximum: 8, controls: 1,
      controlsMaximum: 1, equationNonInterceptTermsMaximum: 50,
    })
    || check.setup?.bootstrap !== "enabled" || check.setup?.samples !== "10000"
    || !check.setup?.profileReady || !check.setup?.scopeExact || !check.setup?.bootstrapScopeExact
    || !check.setup?.previewExact || !check.setup?.previewAccessible
    || check.setup?.unexpectedBlockers?.length !== 0) {
    recordFailure("process-v2-setup-contract", `PROCESS v2 did not preserve its exact graph, profile, bootstrap, scope, and no-unexpected-blocker contract at ${viewport.id}.`, check);
  }
  if (!check.accessibility || !Object.values(check.accessibility).every(Boolean)
    || !check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow
    || !check.dialogBounds?.withinHorizontalViewport || check.dialogBounds?.pageHorizontalOverflow
    || check.completedResult?.available || check.completedResult?.synthesizedByHarness) {
    recordFailure("process-v2-accessibility-truth-layout", `PROCESS v2 exposed an accessibility, focus, overflow, or fabricated-result defect at ${viewport.id}.`, check);
  }
}

async function auditStructuralPathRandomizationDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const check = {
    viewport: viewport.id,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    permutationsInputCount: 0,
    permutationsInputType: null,
    permutationsInputValue: null,
    expectedDefaultPermutations: "999",
    bootstrapSamplesInputCount: 0,
    mutuallyExclusive: false,
    methodDescription: "",
    distinctFromMgaAndMicom: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "structural-path-randomization-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "pls_permutation");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;
    check.methodDescription = compactCalculationText(await calculationOption(dialog, "pls_permutation").textContent());
    check.distinctFromMgaAndMicom = /single-model Freedman(?:\u2013|-|\s)Lane randomization/i.test(check.methodDescription)
      && /structural paths/i.test(check.methodDescription)
      && /fixed original PLS construct scores/i.test(check.methodDescription)
      && /unadjusted pathwise p values/i.test(check.methodDescription)
      && !/\bMGA\b|\bMICOM\b/i.test(check.methodDescription);
    const permutationsInput = dialog.getByLabel("Permutations", { exact: true });
    check.permutationsInputCount = await permutationsInput.count();
    if (check.permutationsInputCount === 1) {
      check.permutationsInputType = await permutationsInput.getAttribute("type");
      check.permutationsInputValue = await permutationsInput.inputValue();
    }
    check.bootstrapSamplesInputCount = await dialog.getByLabel("Bootstrap samples", { exact: true }).count();
    check.mutuallyExclusive = check.permutationsInputCount === 1 && check.bootstrapSamplesInputCount === 0;
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "structural-path-randomization-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.structuralPathRandomization.push(check);
  }
  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("structural-path-randomization-option-contract", "The calculation catalog did not select and link the Structural Path Randomization option at " + viewport.id + ".", check);
  }
  if (!check.distinctFromMgaAndMicom) {
    recordFailure("structural-path-randomization-scope-contract", "Structural Path Randomization did not preserve its required single-model Freedman-Lane structural-path, fixed-score, unadjusted pathwise scope, or it introduced MGA/MICOM group-analysis terminology at " + viewport.id + ".", check);
  }
  if (check.permutationsInputCount !== 1 || check.permutationsInputType !== "number"
    || check.permutationsInputValue !== check.expectedDefaultPermutations) {
    recordFailure("structural-path-randomization-input-contract", "Structural Path Randomization did not expose one Permutations number input with the default value 999 at " + viewport.id + ".", check);
  }
  if (!check.mutuallyExclusive) {
    recordFailure("structural-path-randomization-bootstrap-exclusivity", "Structural Path Randomization did not exclusively replace the Bootstrap samples control at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("structural-path-randomization-dialog-truth-layout", "The idle Structural Path Randomization editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("structural-path-randomization-dialog-close-focus", "Close did not dismiss Structural Path Randomization and restore Calculate focus at " + viewport.id + ".", check);
  }
}

async function auditMgaDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const competenceButton = page.locator('.react-flow__node:has(.smartpls-latent-node)')
    .filter({ hasText: /Competence/i })
    .first();
  const modelContextMenu = page.locator('.nd-context-menu[role="menu"]');
  const comp1Variable = page.locator('.nd-variable-item').filter({ hasText: /^COMP1\b/ }).first();
  const check = {
    viewport: viewport.id,
    preparation: {
      competenceButtonCount: 0,
      deleteCommandCount: 0,
      deleteCommandEnabled: false,
      constructRemoved: false,
      residentGroupingColumnFreed: false,
    },
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    methodDescription: "",
    truthfulScopeDescription: false,
    groupsCategoryCount: 0,
    groupsCategoryLabel: "",
    keyboardSteps: [],
    keyboardContract: false,
    groupingControlCount: 0,
    groupingOptions: [],
    selectedGroupingColumn: null,
    groupAControlCount: 0,
    groupBControlCount: 0,
    groupAValue: null,
    groupBValue: null,
    groupAOption: "",
    groupBOption: "",
    selectedGroupCompleteCases: [],
    permutationInputCount: 0,
    permutationInputType: null,
    permutationInputValue: null,
    permutationInputMin: null,
    permutationInputMax: null,
    weightingScheme: "",
    resultData: "",
    confidenceInputCount: 0,
    confidenceInputValue: null,
    confidenceInputMin: null,
    confidenceInputMax: null,
    confidenceInputStep: null,
    testPlan: "",
    measurementInvariance: "",
    configuralCheckboxCount: 0,
    configuralInitiallyChecked: null,
    configuralCheckedAfterConfirmation: null,
    blockerTextBeforeConfirmation: "",
    blockerTextAfterConfirmation: "",
    lowSampleObserved: false,
    startCommandCount: 0,
    startCommandDisabledBeforeConfirmation: null,
    startCommandDisabledAfterConfirmation: null,
    truthAndOverflow: null,
    closeFocus: null,
    modelRestore: {
      undoCommandCount: 0,
      undoCommandEnabled: false,
      competenceRestored: false,
      comp1AssignmentRestored: false,
    },
  };
  let modelChanged = false;

  try {
    check.preparation.competenceButtonCount = await competenceButton.count();
    if (check.preparation.competenceButtonCount === 1) {
      await competenceButton.click();
      await competenceButton.focus();
      await competenceButton.click({ button: "right" });
      await modelContextMenu.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
      const deleteSelection = modelContextMenu.getByRole("menuitem", { name: /^Delete Selection/i });
      check.preparation.deleteCommandCount = await deleteSelection.count();
      check.preparation.deleteCommandEnabled = check.preparation.deleteCommandCount === 1
        ? await deleteSelection.isEnabled().catch(() => false)
        : false;
      if (check.preparation.deleteCommandEnabled) {
        await deleteSelection.click();
        await page.waitForFunction(() => (
          !Array.from(document.querySelectorAll('.react-flow__node:has(.smartpls-latent-node)'))
            .some((node) => /Competence/i.test((node.textContent ?? "").trim()))
          && Array.from(document.querySelectorAll('button.nd-variable-item'))
            .some((button) => /^COMP1\b/.test((button.textContent ?? "").trim()) && !button.classList.contains("assigned"))
        ), undefined, { timeout: 1_000 }).catch(() => null);
        check.preparation.constructRemoved = await competenceButton.count() === 0;
        check.preparation.residentGroupingColumnFreed = await comp1Variable.evaluate((node) => !node.classList.contains("assigned")).catch(() => false);
        modelChanged = check.preparation.constructRemoved && check.preparation.residentGroupingColumnFreed;
      }
    }

    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "mga-dialog-open");
    if (check.dialogOpened) {
      const selection = await selectCalculationMethod(dialog, "mga");
      check.pointerSelected = selection.pointerSelected;
      check.linkage = selection.linkage;
      check.methodDescription = compactCalculationText(await calculationOption(dialog, "mga").textContent());
      check.truthfulScopeDescription = /MICOM measurement(?:-| )invariance/i.test(check.methodDescription)
        && /Group A minus Group B paths, loadings, and weights/i.test(check.methodDescription);
      const groupsCategory = dialog.locator('#nd-calculation-method-list [role="group"][aria-labelledby="nd-calculation-category-groups"]');
      check.groupsCategoryCount = await groupsCategory.count();
      check.groupsCategoryLabel = compactCalculationText(await dialog.locator("#nd-calculation-category-groups").textContent().catch(() => ""));

      const mgaOption = calculationOption(dialog, "mga");
      await mgaOption.focus();
      for (const expected of [
        { key: "ArrowDown", focusedKind: "predict", selectedKind: "mga" },
        { key: "ArrowUp", focusedKind: "mga", selectedKind: "mga" },
        { key: "Enter", focusedKind: "mga", selectedKind: "mga" },
      ]) {
        await page.keyboard.press(expected.key);
        const expectedFocusedId = "nd-calculation-method-" + expected.focusedKind;
        const expectedSelectedId = "nd-calculation-method-" + expected.selectedKind;
        await page.waitForFunction(({ focusedId, selectedId }) => (
          document.activeElement?.id === focusedId
          && document.querySelector('#nd-calculation-method-list [role="option"][aria-selected="true"]')?.id === selectedId
        ), { focusedId: expectedFocusedId, selectedId: expectedSelectedId }, { timeout: 1_000 }).catch(() => null);
        check.keyboardSteps.push(await page.evaluate(({ key, focusedId, selectedId }) => {
          const options = Array.from(document.querySelectorAll('#nd-calculation-method-list [role="option"]'));
          return {
            key,
            expectedFocusedId: focusedId,
            expectedSelectedId: selectedId,
            focusedId: document.activeElement?.id ?? null,
            selectedId: document.querySelector('#nd-calculation-method-list [role="option"][aria-selected="true"]')?.id ?? null,
            tabStopIds: options.filter((option) => option.tabIndex === 0).map((option) => option.id),
          };
        }, { key: expected.key, focusedId: expectedFocusedId, selectedId: expectedSelectedId }));
      }
      check.keyboardContract = check.keyboardSteps.length === 3 && check.keyboardSteps.every((step) => (
        step.focusedId === step.expectedFocusedId
        && step.selectedId === step.expectedSelectedId
        && step.tabStopIds.length === 1
        && step.tabStopIds[0] === step.expectedFocusedId
      ));

      const grouping = dialog.locator("#nd-calculation-group-column");
      check.groupingControlCount = await grouping.count();
      if (check.groupingControlCount === 1) {
        check.groupingOptions = (await grouping.locator("option").allTextContents()).map(compactCalculationText);
        const comp1OptionCount = await grouping.locator('option[value="COMP1"]:not([disabled])').count();
        if (modelChanged && comp1OptionCount === 1) {
          await grouping.selectOption("COMP1");
          await page.waitForFunction(() => {
            const groupA = document.querySelector("#nd-calculation-group-a");
            const groupB = document.querySelector("#nd-calculation-group-b");
            return groupA instanceof HTMLSelectElement && groupA.value
              && groupB instanceof HTMLSelectElement && groupB.value
              && groupA.value !== groupB.value;
          }, undefined, { timeout: 1_000 }).catch(() => null);
          check.selectedGroupingColumn = await grouping.inputValue().catch(() => null);
        }
      }

      const groupA = dialog.locator("#nd-calculation-group-a");
      const groupB = dialog.locator("#nd-calculation-group-b");
      check.groupAControlCount = await groupA.count();
      check.groupBControlCount = await groupB.count();
      if (check.groupAControlCount === 1 && check.groupBControlCount === 1) {
        check.groupAValue = await groupA.inputValue();
        check.groupBValue = await groupB.inputValue();
        check.groupAOption = compactCalculationText(await groupA.locator("option:checked").textContent().catch(() => ""));
        check.groupBOption = compactCalculationText(await groupB.locator("option:checked").textContent().catch(() => ""));
        check.selectedGroupCompleteCases = [check.groupAOption, check.groupBOption].map((label) => {
          const match = label.match(/([\d,]+) complete of/i);
          return match ? Number(match[1].replace(/,/g, "")) : null;
        });
        check.lowSampleObserved = check.selectedGroupCompleteCases.some((count) => typeof count === "number" && count < 10);
      }

      const weightingNote = dialog.locator(".nd-setting-note").filter({ hasText: /^Weighting scheme/ });
      const resultDataNote = dialog.locator(".nd-setting-note").filter({ hasText: /^Result data/ });
      check.weightingScheme = compactCalculationText(await weightingNote.locator("strong").textContent().catch(() => ""));
      check.resultData = compactCalculationText(await resultDataNote.locator("strong").textContent().catch(() => ""));

      const permutations = dialog.locator("#nd-calculation-group-permutations");
      check.permutationInputCount = await permutations.count();
      if (check.permutationInputCount === 1) {
        check.permutationInputType = await permutations.getAttribute("type");
        check.permutationInputValue = await permutations.inputValue();
        check.permutationInputMin = await permutations.getAttribute("min");
        check.permutationInputMax = await permutations.getAttribute("max");
      }
      const confidence = dialog.locator("#nd-calculation-micom-confidence");
      check.confidenceInputCount = await confidence.count();
      if (check.confidenceInputCount === 1) {
        check.confidenceInputValue = await confidence.inputValue();
        check.confidenceInputMin = await confidence.getAttribute("min");
        check.confidenceInputMax = await confidence.getAttribute("max");
        check.confidenceInputStep = await confidence.getAttribute("step");
      }
      const testNote = dialog.locator(".nd-setting-note").filter({ hasText: /^Test/ });
      check.testPlan = normalizeGroupDifference(await testNote.locator("strong").textContent().catch(() => ""));
      const measurementNote = dialog.locator(".nd-setting-note").filter({ hasText: /^Measurement invariance/ });
      check.measurementInvariance = compactCalculationText(await measurementNote.locator("strong").textContent().catch(() => ""));
      const configural = dialog.locator("#nd-calculation-micom-configural");
      check.configuralCheckboxCount = await configural.count();
      check.configuralInitiallyChecked = check.configuralCheckboxCount === 1 ? await configural.isChecked() : null;
      const blocker = dialog.locator('.nd-blocker[role="alert"]');
      check.blockerTextBeforeConfirmation = compactCalculationText(await blocker.textContent().catch(() => ""));
      const start = dialog.getByRole("button", { name: "Start group analysis", exact: true });
      check.startCommandCount = await start.count();
      check.startCommandDisabledBeforeConfirmation = check.startCommandCount === 1 ? await start.isDisabled() : null;
      if (check.configuralCheckboxCount === 1) {
        await configural.check();
        await page.waitForFunction(() => {
          const alert = document.querySelector('.nd-dialog-calculation .nd-blocker[role="alert"]');
          return !/Confirm MICOM Step 1/i.test(alert?.textContent ?? "");
        }, undefined, { timeout: 1_000 }).catch(() => null);
        check.configuralCheckedAfterConfirmation = await configural.isChecked();
        check.blockerTextAfterConfirmation = compactCalculationText(await blocker.textContent().catch(() => ""));
        check.startCommandDisabledAfterConfirmation = await start.isDisabled();
      }
      check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
      await capture(page, "mga-dialog", sequence, viewport, { dialog: "calculation" });
      if (check.configuralCheckboxCount === 1) await configural.uncheck();
      check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
    }
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => check.closeFocus);
    }
    if (modelChanged) {
      const editMenu = page.locator('.nd-menubar [role="menuitem"][aria-haspopup="menu"]').filter({ hasText: /^Edit$/ });
      const editMenuCount = await editMenu.count();
      let undo = null;
      if (editMenuCount === 1) {
        await editMenu.click();
        const popupId = await editMenu.getAttribute("aria-controls");
        const popup = popupId ? page.locator("#" + popupId) : page.locator('.nd-menu-popup[role="menu"]');
        await popup.waitFor({ state: "visible", timeout: 1_000 }).catch(() => null);
        undo = popup.getByRole("menuitem", { name: /^Undo/i });
      }
      check.modelRestore.undoCommandCount = undo ? await undo.count() : 0;
      check.modelRestore.undoCommandEnabled = undo && check.modelRestore.undoCommandCount === 1
        ? await undo.isEnabled().catch(() => false)
        : false;
      if (check.modelRestore.undoCommandEnabled) {
        await undo.click();
        await page.waitForFunction(() => (
          Array.from(document.querySelectorAll('.react-flow__node:has(.smartpls-latent-node)'))
            .some((node) => /Competence/i.test((node.textContent ?? "").trim()))
          && Array.from(document.querySelectorAll('button.nd-variable-item'))
            .some((button) => /^COMP1\b/.test((button.textContent ?? "").trim()) && button.classList.contains("assigned"))
        ), undefined, { timeout: 1_000 }).catch(() => null);
        check.modelRestore.competenceRestored = await competenceButton.count() === 1;
        check.modelRestore.comp1AssignmentRestored = await comp1Variable.evaluate((node) => node.classList.contains("assigned")).catch(() => false);
      }
    }
    evidence.checks.mga.push(check);
  }

  if (!check.preparation.constructRemoved || !check.preparation.residentGroupingColumnFreed) {
    recordFailure("mga-resident-sample-preparation", "The real model commands did not free resident sample column COMP1 for truthful two-group setup inspection at " + viewport.id + ".", check);
  }
  if (!check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage
    || check.groupsCategoryCount !== 1 || check.groupsCategoryLabel !== "Groups") {
    recordFailure("mga-groups-catalog-contract", "MICOM and Two-Group Permutation MGA was not selected and linked under the Groups category at " + viewport.id + ".", check);
  }
  if (!check.truthfulScopeDescription) {
    recordFailure("mga-method-scope-contract", "The group-analysis catalog option did not disclose joint MICOM measurement invariance plus Group A-minus-B path, loading, and weight comparisons at " + viewport.id + ".", check);
  }
  if (!check.keyboardContract) {
    recordFailure("mga-listbox-keyboard-contract", `The MGA option did not preserve roving focus, selection, and Enter behavior in the ${nativeCalculationMethods.length}-method listbox at ${viewport.id}.`, check);
  }
  if (check.groupingControlCount !== 1 || check.selectedGroupingColumn !== "COMP1"
    || check.groupAControlCount !== 1 || check.groupBControlCount !== 1
    || !check.groupAValue || !check.groupBValue || check.groupAValue === check.groupBValue
    || check.selectedGroupCompleteCases.some((count) => typeof count !== "number")) {
    recordFailure("mga-explicit-group-controls", "The MGA editor did not profile resident COMP1 data into distinct, counted Group A and Group B controls at " + viewport.id + ".", check);
  }
  if (check.permutationInputCount !== 1 || check.permutationInputType !== "number"
    || check.permutationInputValue !== "5000" || check.permutationInputMin !== "5000" || check.permutationInputMax !== "10000") {
    recordFailure("mga-permutation-default-contract", "The MICOM/MGA editor did not expose the bounded default and minimum of 5,000 permutations at " + viewport.id + ".", check);
  }
  if (check.weightingScheme !== "Path weighting (fixed)" || check.resultData !== "Standardized (fixed)") {
    recordFailure("mga-fixed-estimation-scope", "The MICOM/MGA editor did not fix path weighting and standardized result data at " + viewport.id + ".", check);
  }
  if (check.confidenceInputCount !== 1 || check.confidenceInputValue !== "95"
    || check.confidenceInputMin !== "80" || check.confidenceInputMax !== "99.9" || check.confidenceInputStep !== "0.1") {
    recordFailure("mga-micom-confidence-contract", "The MICOM/MGA editor did not expose one bounded 95% confidence input at " + viewport.id + ".", check);
  }
  if (check.testPlan !== "Two-tailed; Group A - Group B") {
    recordFailure("mga-fixed-test-contract", "The MGA editor did not fix the comparison to a two-tailed Group A minus Group B test at " + viewport.id + ".", check);
  }
  if (!/Step 2 composition and Step 3 pooled-score means and variances are tested with the same deterministic permutations\./i.test(check.measurementInvariance)
    || check.configuralCheckboxCount !== 1
    || check.configuralInitiallyChecked !== false
    || check.configuralCheckedAfterConfirmation !== true
    || !/Confirm MICOM Step 1: identical indicators, coding, data treatment, algorithm settings, and substantive meaning across both groups/i.test(check.blockerTextBeforeConfirmation)
    || /Confirm MICOM Step 1/i.test(check.blockerTextAfterConfirmation)
    || check.startCommandDisabledBeforeConfirmation !== true) {
    recordFailure("mga-micom-step-contract", "The MICOM/MGA editor did not require explicit Step 1 confirmation before enabling automatic Steps 2 and 3 at " + viewport.id + ".", check);
  }
  if (check.lowSampleObserved && (check.startCommandCount !== 1 || check.startCommandDisabledAfterConfirmation !== true
    || !/Group A has \d+ complete model cases; at least 10 are required/i.test(check.blockerTextAfterConfirmation)
    || !/Group B has \d+ complete model cases; at least 10 are required/i.test(check.blockerTextAfterConfirmation))) {
    recordFailure("mga-small-group-blocker", "Resident groups below ten complete model cases did not produce explicit A/B blockers and a disabled Start command at " + viewport.id + ".", check);
  }
  if (!check.lowSampleObserved && check.startCommandDisabledAfterConfirmation !== false) {
    recordFailure("mga-configural-ready-transition", "The valid MICOM/MGA setup did not become runnable after explicit Step 1 confirmation at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("mga-dialog-truth-layout", "The idle MGA editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("mga-dialog-close-focus", "Close did not dismiss MGA and restore Calculate focus at " + viewport.id + ".", check);
  }
  if (modelChanged && (!check.modelRestore.competenceRestored || !check.modelRestore.comp1AssignmentRestored)) {
    recordFailure("mga-model-fixture-restore", "Undo did not restore the real Competence construct and COMP1 indicator assignment after MGA inspection at " + viewport.id + ".", check);
  }
}

async function auditPredictionDialog(page, viewport, sequence, trigger) {
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  const expectedPlan = "Complete cases; seeded balanced 10-fold x 10-repeat cross-validation; deterministic modulo-4 holdout retained as a secondary check";
  const check = {
    viewport: viewport.id,
    genericTriggerLabel: trigger ? compactCalculationText(await trigger.textContent().catch(() => "")) : "",
    genericCalculateTrigger: false,
    dialogOpened: false,
    pointerSelected: false,
    linkage: null,
    startPredictionCount: 0,
    predictionPlan: "",
    predictionTargets: "",
    predictionBenchmarks: "",
    cvpatScope: "",
    seed: null,
    expectedPlan,
    excludedControlCounts: {},
    excludedControlsAbsent: false,
    truthAndOverflow: null,
    closeFocus: null,
  };
  check.genericCalculateTrigger = isGenericCalculateLabel(check.genericTriggerLabel);
  try {
    check.dialogOpened = await reopenCalculationDialog(page, viewport, trigger, "prediction-dialog-open");
    if (!check.dialogOpened) return;
    const selection = await selectCalculationMethod(dialog, "predict");
    check.pointerSelected = selection.pointerSelected;
    check.linkage = selection.linkage;
    check.startPredictionCount = await dialog.getByRole("button", { name: "Start prediction", exact: true }).count();
    check.predictionPlan = normalizeCalculationPlan(await dialog.locator("#nd-calculation-prediction-plan strong").textContent().catch(() => ""));
    check.predictionTargets = compactCalculationText(await dialog.locator("#nd-calculation-prediction-targets strong").textContent().catch(() => ""));
    check.predictionBenchmarks = compactCalculationText(await dialog.locator("#nd-calculation-prediction-benchmarks strong").textContent().catch(() => ""));
    check.cvpatScope = compactCalculationText(await dialog.locator("#nd-calculation-prediction-cvpat strong").textContent().catch(() => ""));
    const seedInput = dialog.locator("#nd-calculation-seed");
    check.seed = await seedInput.count() === 1 ? Number(await seedInput.inputValue()) : null;
    check.excludedControlCounts = {
      bootstrapSamples: await dialog.getByLabel("Bootstrap samples", { exact: true }).count(),
      permutations: await dialog.getByLabel("Permutations", { exact: true }).count(),
      parallelWorkers: await dialog.getByLabel("Parallel workers", { exact: true }).count(),
    };
    check.excludedControlsAbsent = Object.values(check.excludedControlCounts).every((count) => count === 0);
    check.truthAndOverflow = await inspectCalculationTruthAndOverflow(dialog);
    await capture(page, "prediction-dialog", sequence, viewport, { dialog: "calculation" });
    check.closeFocus = await closeCalculationAndCheckFocus(page, dialog, trigger);
  } finally {
    if (await dialog.isVisible().catch(() => false)) {
      await closeCalculationAndCheckFocus(page, dialog, trigger).catch(() => null);
    }
    evidence.checks.prediction.push(check);
  }
  if (!check.genericCalculateTrigger || !check.dialogOpened || !check.pointerSelected || !check.linkage?.linkage) {
    recordFailure("prediction-generic-calculate-option", "Prediction was not selected and linked from the generic Calculate catalog at " + viewport.id + ".", check);
  }
  if (check.startPredictionCount !== 1) {
    recordFailure("prediction-start-command", "Prediction did not expose exactly one Start prediction command at " + viewport.id + ".", check);
  }
  if (!check.excludedControlsAbsent || !Number.isInteger(check.seed) || check.seed < 0) {
    recordFailure("prediction-settings-contract", "Prediction did not expose one recorded seed while keeping unrelated resampling and worker controls hidden at " + viewport.id + ".", check);
  }
  if (check.predictionPlan !== expectedPlan
    || !/endogenous indicators.*primary.*construct-score metrics.*supplementary/i.test(check.predictionTargets)
    || !/indicator average \(IA\).*linear model \(LM(?:, where estimable)?\)/i.test(check.predictionBenchmarks)
    || !/single fitted model versus IA\/LM benchmarks/i.test(check.cvpatScope)
    || !/one-sided test, 95% confidence/i.test(check.cvpatScope)
    || !/not a comparison of saved models/i.test(check.cvpatScope)) {
    recordFailure("prediction-v2-scope-disclosure", "Prediction did not disclose the exact seeded 10x10 indicator, IA/LM, one-sided CVPAT, and single-model scope at " + viewport.id + ".", check);
  }
  if (!check.truthAndOverflow?.noFabricatedRunState || !check.truthAndOverflow?.noHorizontalOverflow) {
    recordFailure("prediction-dialog-truth-layout", "The idle Prediction editor exposed fabricated run state or horizontal overflow at " + viewport.id + ".", check);
  }
  if (!check.closeFocus?.dialogClosed || !check.closeFocus?.focusRestored) {
    recordFailure("prediction-dialog-close-focus", "Close did not dismiss Prediction and restore generic Calculate focus at " + viewport.id + ".", check);
  }
}

async function attemptTruthfulRunningCapture(page, viewport) {
  const probe = await page.evaluate(() => ({
    fixtureApiPresent: typeof window.__QUICKPLS_SMOKE__?.setRunMonitorFixture === "function",
    visibleRunningState: Boolean(document.querySelector(".nd-run-progress.running")),
    nativeRuntimePresent: Boolean(window.__TAURI_INTERNALS__),
  }));

  if (probe.visibleRunningState) {
    recordFailure("unexpected-browser-running-state", `The production web preview exposed a running calculation at ${viewport.id} without packaged-Tauri job evidence.`, probe);
  }
  recordSkip("running-calculation", "A browser-preview fixture cannot prove a genuine in-flight native calculation. The harness did not invoke setRunMonitorFixture and did not create a running screenshot.", {
    state: "running-calculation",
    viewport: viewport.id,
    fixtureApiPresent: probe.fixtureApiPresent,
    requiredNativeFollowUp: "Capture a genuine in-flight PLS job in the packaged Tauri app after backend/project synchronization is verified.",
  });
}

async function visualStep(viewport, label, operation, timeoutMs = 60_000) {
  console.log(`[visual:${viewport.id}] start ${label}`);
  let timeout;
  try {
    const result = await Promise.race([
      operation(),
      new Promise((_, reject) => {
        timeout = setTimeout(() => reject(new Error(`Visual step timed out after ${timeoutMs} ms: ${viewport.id} / ${label}`)), timeoutMs);
      }),
    ]);
    console.log(`[visual:${viewport.id}] pass ${label}`);
    return result;
  } finally {
    if (timeout) clearTimeout(timeout);
  }
}

async function exerciseViewport(browser, viewport) {
  const page = await browser.newPage({
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: 1,
  });
  page.setDefaultTimeout(3_000);
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "quickpls:native-ui-preferences:v1",
      JSON.stringify({ experimentalLabsEnabled: true }),
    );
  });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push({ viewport: viewport.id, type: "pageerror", message: error.message }));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push({ viewport: viewport.id, type: "console", message: message.text() });
  });

  try {
    await openSmokePage(page, "1");

    await setSurface(page, "launcher");
    await capture(page, "launcher", 1, viewport);
    await auditKeyboardAndFocus(page, viewport);
    await auditWorkspaceExplorer(page, viewport, 1);

    await setSurface(page, "data");
    await capture(page, "data", 2, viewport);
    await auditRecodeDialog(page, viewport, 3);
    await auditImportDataDialog(page, viewport, 4);

    await setSurface(page, "model");
    await capture(page, "model", 5, viewport);
    await auditModelContextMenuParity(page, viewport);
    await auditModeratingEffectDialog(page, viewport, 6);
    await auditHigherOrderConstructDialog(page, viewport, 6);

    // The HOC audit intentionally starts from a data-only fixture. Restore the
    // canonical sample before inspecting model-bound calculation methods.
    await visualStep(viewport, "restore canonical smoke page after HOC", () => openSmokePage(page, "1"));
    await visualStep(viewport, "restore canonical model surface after HOC", () => setSurface(page, "model"));

    const calculation = await visualStep(viewport, "open calculation dialog", () => openCalculationDialogForInspection(page, viewport));
    try {
      if (calculation.opened) {
        await visualStep(viewport, "calculation catalogue", () => auditCalculationCatalogDialog(page, viewport, 6, calculation.trigger, calculation.commandSurface));
        await visualStep(viewport, "PLS-C dialog", () => auditPlscDialog(page, viewport, 7, calculation.trigger));
        await visualStep(viewport, "weighted PLS dialog", () => auditWplsDialog(page, viewport, 8, calculation.trigger));
        await visualStep(viewport, "GSCA dialog", () => auditGscaDialog(page, viewport, 9, calculation.trigger));
        await visualStep(viewport, "CCA dialog", () => auditCcaDialog(page, viewport, 9, calculation.trigger));
        if (nativeCalculationMethods.some((method) => method.kind === "cta_pls")) {
          await visualStep(viewport, "CTA-PLS dialog", () => auditCtaPlsDialog(page, viewport, 9, calculation.trigger));
        }
        await visualStep(viewport, "CB-SEM dialog", () => auditCbsemDialog(page, viewport, 10, calculation.trigger));
        await visualStep(viewport, "IPMA dialog", () => auditIpmaDialog(page, viewport, 10, calculation.trigger));
        await visualStep(viewport, "structural path randomization dialog", () => auditStructuralPathRandomizationDialog(page, viewport, 11, calculation.trigger));
        await visualStep(viewport, "prediction dialog", () => auditPredictionDialog(page, viewport, 12, calculation.trigger));
        await visualStep(viewport, "MGA dialog", () => auditMgaDialog(page, viewport, 13, calculation.trigger));
      }
    } finally {
      await visualStep(viewport, "close calculation dialog", () => calculation.restore());
    }

    await visualStep(viewport, "NCA standalone dialog", () => auditNcaStandaloneDialogFromData(page, viewport, 14));
    await visualStep(viewport, "PCA standalone dialog", () => auditPcaStandaloneDialogFromData(page, viewport, 16));
    await visualStep(viewport, "OLS standalone dialog", () => auditOlsStandaloneDialogFromData(page, viewport, 17));
    await visualStep(viewport, "logistic standalone dialog", () => auditLogisticStandaloneDialogFromData(page, viewport, 18));
    await visualStep(viewport, "regression bootstrap dialogs", () => auditRegressionBootstrapDialogFromData(page, viewport, 19, 20));
    await visualStep(viewport, "PROCESS v2 dialog", () => auditProcessV2DialogFromImportedProject(page, viewport, 21));

    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject());
    await setSurface(page, "results");
    await page.getByText("No completed calculation", { exact: true }).waitFor();
    await capture(page, "empty-results", 18, viewport);

    // `completed` is the repository's existing deterministic Corporate
    // Reputation result fixture. No synthetic table rows or ad-hoc run are added.
    await openSmokePage(page, "completed");
    await setSurface(page, "results");
    await page.waitForSelector(".nd-result-tree");
    await page.waitForSelector(".nd-result-table, .nd-result-diagram-view");
    await capture(page, "completed-results", 17, viewport);

    await auditCompletedMediationResults(page, viewport, 18);
    await auditCompletedModerationResultsIfAvailable(page, viewport, 19);

    await page.locator(".nd-commandbar button", { hasText: "Export" }).first().click();
    await page.waitForSelector('.nd-dialog-export[role="dialog"]');
    await capture(page, "export-dialog", 18, viewport, { dialog: "export" });
  } finally {
    evidence.consoleErrors.push(...pageErrors);
    await page.close();
  }
}

async function exercise200PercentScale(browser) {
  const page = await browser.newPage({
    viewport: { width: scale200Viewport.width, height: scale200Viewport.height },
    deviceScaleFactor: scale200Viewport.deviceScaleFactor,
  });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push({ viewport: scale200Viewport.id, type: "pageerror", message: error.message }));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push({ viewport: scale200Viewport.id, type: "console", message: message.text() });
  });

  try {
    await openSmokePage(page, "1");
    const observed = await page.evaluate(() => ({
      devicePixelRatio: window.devicePixelRatio,
      cssViewport: { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight },
      physicalViewportEstimate: {
        width: Math.round(document.documentElement.clientWidth * window.devicePixelRatio),
        height: Math.round(document.documentElement.clientHeight * window.devicePixelRatio),
      },
    }));
    const auditedStates = [];
    for (const surface of ["launcher", "data", "model"]) {
      await setSurface(page, surface);
      if (surface === "model") {
        await capture(page, "model-200pct-scale", 10, scale200Viewport);
      } else {
        await inspectShellState(page, `${surface}-200pct-scale`, scale200Viewport);
        await inspectAccessibility(page, `${surface}-200pct-scale`, scale200Viewport);
      }
      auditedStates.push(surface);
    }
    await openSmokePage(page, "completed");
    await setSurface(page, "results");
    await page.waitForSelector(".nd-result-tree");
    await inspectShellState(page, "results-200pct-scale", scale200Viewport);
    await inspectAccessibility(page, "results-200pct-scale", scale200Viewport);
    auditedStates.push("results");

    evidence.checks.scale200Percent = {
      requestedDeviceScaleFactor: 2,
      observed,
      auditedStates,
      browserChromeZoomControlled: false,
      interpretation: "Verified Chromium rendering and layout at DPR 2 while retaining a 1024x700 CSS viewport. Playwright does not expose browser-chrome zoom and this is not evidence of Windows display scaling.",
    };
    if (Math.abs(observed.devicePixelRatio - 2) > 0.01) {
      recordFailure("device-scale-200-not-applied", "Chromium did not apply the requested 200% device scale.", evidence.checks.scale200Percent);
    }
    recordSkip("browser-chrome-zoom", "Playwright does not expose browser-chrome zoom controls. DPR 2 layout is audited separately and is not relabeled as native Windows or browser-chrome zoom evidence.", {
      requiredNativeFollowUp: "Verify the packaged Tauri window at 200% Windows display scaling.",
    });
  } finally {
    evidence.consoleErrors.push(...pageErrors);
    await page.close();
  }
}

async function exerciseLargeModelFixture(browser) {
  const viewport = { ...viewports[2], id: "1440x900-large-model" };
  const page = await browser.newPage({
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: 1,
  });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push({ viewport: viewport.id, type: "pageerror", message: error.message }));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push({ viewport: viewport.id, type: "console", message: message.text() });
  });

  const modelCounts = () => page.evaluate(() => window.__QUICKPLS_SMOKE__?.modelCounts?.() ?? ({ constructs: 0, indicators: 0 }));

  try {
    await openSmokePage(page, "1");
    await setSurface(page, "model");
    const apiPresent = await page.evaluate(() => typeof window.__QUICKPLS_SMOKE__?.loadDiagramFixture === "function");
    if (!apiPresent) {
      evidence.checks.largeModel = { supported: false, target: largeModelTarget, reason: "loadDiagramFixture is not exposed by the smoke API." };
      recordSkip("large-model-interaction-performance", "The smoke API does not expose a model fixture loader; no 20-construct/80-indicator state was fabricated.", {
        target: largeModelTarget,
      });
      return;
    }

    const renderStarted = performance.now();
    const apiReturn = await page.evaluate(async () => {
      const value = await window.__QUICKPLS_SMOKE__?.loadDiagramFixture("large");
      return value == null ? null : typeof value;
    });
    await page.waitForFunction(({ constructs, indicators }) => {
      const counts = window.__QUICKPLS_SMOKE__?.modelCounts?.();
      return counts != null && counts.constructs >= constructs && counts.indicators >= indicators;
    }, largeModelTarget, { timeout: interactionBudgetsMs.fixtureRender }).catch(() => null);
    const fixtureRenderMs = Math.round((performance.now() - renderStarted) * 10) / 10;
    const counts = await modelCounts();
    if (counts.constructs < largeModelTarget.constructs || counts.indicators < largeModelTarget.indicators) {
      evidence.checks.largeModel = {
        supported: false,
        apiPresent: true,
        apiReturn,
        requestedFixture: "large",
        target: largeModelTarget,
        observed: counts,
        waitedMs: fixtureRenderMs,
        reason: "The existing fixture loader did not produce the required model size.",
      };
      recordSkip("large-model-interaction-performance", "The existing smoke fixture did not produce at least 20 constructs and 80 indicators; no nodes were synthesized by the harness.", {
        target: largeModelTarget,
        observed: counts,
      });
      return;
    }

    const targetConstructId = "construct-1";
    await page.evaluate((id) => {
      window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id } }));
    }, targetConstructId);
    await page.waitForTimeout(320);
    const firstFlowNode = page.locator(`.react-flow__node[data-id="${targetConstructId}"]`).first();
    await firstFlowNode.waitFor({ state: "visible", timeout: interactionBudgetsMs.selection });
    const firstConstruct = firstFlowNode.locator(".smartpls-latent-node");
    const selectionStarted = performance.now();
    await firstConstruct.click({ timeout: interactionBudgetsMs.selection });
    await page.waitForFunction((id) => document.querySelector(`.react-flow__node[data-id="${id}"].selected, .react-flow__node[data-id="${id}"] .smartpls-latent-node.selected`) != null, targetConstructId, { timeout: interactionBudgetsMs.selection });
    const selectionMs = Math.round((performance.now() - selectionStarted) * 10) / 10;
    const selectionChanged = await firstFlowNode.evaluate((node) => node.classList.contains("selected") || node.querySelector(".smartpls-latent-node.selected") != null);

    const beforeDrag = await firstFlowNode.boundingBox();
    let dragChanged = false;
    let dragMs = null;
    if (beforeDrag) {
      const dragStarted = performance.now();
      const startX = beforeDrag.x + beforeDrag.width / 2;
      const startY = beforeDrag.y + beforeDrag.height / 2;
      await page.mouse.move(startX, startY);
      await page.mouse.down();
      await page.mouse.move(startX + 48, startY + 32, { steps: 8 });
      await page.mouse.up();
      await page.waitForTimeout(80);
      dragMs = Math.round((performance.now() - dragStarted) * 10) / 10;
      const afterDrag = await firstFlowNode.boundingBox();
      dragChanged = Boolean(afterDrag && Math.hypot(afterDrag.x - beforeDrag.x, afterDrag.y - beforeDrag.y) >= 4);
    }

    const viewportTransform = () => page.locator(".react-flow__viewport").evaluate((node) => getComputedStyle(node).transform);
    const panButton = page.locator(".nd-commandbar button").filter({ hasText: /^Pan$/ }).first();
    let panChanged = false;
    let panMs = null;
    if (await panButton.isEnabled().catch(() => false)) {
      await panButton.click();
      await page.waitForTimeout(40);
      const safePoint = await page.evaluate(() => {
        const pane = document.querySelector(".react-flow__pane");
        if (!(pane instanceof HTMLElement)) return null;
        const rect = pane.getBoundingClientRect();
        const candidates = [[0.9, 0.9], [0.1, 0.9], [0.9, 0.2], [0.5, 0.9]];
        for (const [xRatio, yRatio] of candidates) {
          const x = rect.left + rect.width * xRatio;
          const y = rect.top + rect.height * yRatio;
          const hit = document.elementFromPoint(x, y);
          if (hit && !hit.closest(".react-flow__node, .react-flow__edge, .canvas-toolbar, .react-flow__controls")) return { x, y };
        }
        return null;
      });
      if (safePoint) {
        const beforePan = await viewportTransform();
        const panStarted = performance.now();
        await page.mouse.move(safePoint.x, safePoint.y);
        await page.mouse.down();
        await page.mouse.move(safePoint.x + 54, safePoint.y + 36, { steps: 8 });
        await page.mouse.up();
        await page.waitForTimeout(80);
        panMs = Math.round((performance.now() - panStarted) * 10) / 10;
        panChanged = (await viewportTransform()) !== beforePan;
      }
    }

    const zoomButton = page.locator(".react-flow__controls-zoomin").first();
    const beforeZoom = await viewportTransform();
    const zoomStarted = performance.now();
    await zoomButton.click();
    await page.waitForTimeout(280);
    const zoomMs = Math.round((performance.now() - zoomStarted) * 10) / 10;
    const zoomChanged = (await viewportTransform()) !== beforeZoom;

    const switchStarted = performance.now();
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("data"));
    await page.waitForSelector('.nd-app[data-surface="data"]');
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.setView("model"));
    await page.waitForSelector('.nd-app[data-surface="model"]');
    await page.waitForFunction(({ constructs, indicators }) => {
      const counts = window.__QUICKPLS_SMOKE__?.modelCounts?.();
      return counts != null && counts.constructs >= constructs && counts.indicators >= indicators;
    }, largeModelTarget, { timeout: interactionBudgetsMs.workspaceRoundTrip }).catch(() => null);
    const retainedCounts = await modelCounts();
    const workspaceRoundTripMs = Math.round((performance.now() - switchStarted) * 10) / 10;

    const preflightStarted = performance.now();
    const preflight = await page.evaluate(() => window.__QUICKPLS_SMOKE__?.modelPreflight?.() ?? null).catch(() => null);
    const preflightMs = Math.round((performance.now() - preflightStarted) * 10) / 10;
    const preflightPresent = Boolean(preflight && [preflight.ready, preflight.blockers, preflight.warnings].every(Number.isInteger)
      && preflight.ready + preflight.blockers + preflight.warnings > 0);

    const reopenStarted = performance.now();
    await page.evaluate(async () => {
      await window.__QUICKPLS_SMOKE__?.loadEmptyProject?.();
      await window.__QUICKPLS_SMOKE__?.loadDiagramFixture?.("large");
    });
    await page.waitForFunction(({ constructs, indicators }) => {
      const counts = window.__QUICKPLS_SMOKE__?.modelCounts?.();
      return counts != null && counts.constructs >= constructs && counts.indicators >= indicators;
    }, largeModelTarget, { timeout: interactionBudgetsMs.fixtureReopen }).catch(() => null);
    const reopenedCounts = await modelCounts();
    const fixtureReopenMs = Math.round((performance.now() - reopenStarted) * 10) / 10;
    const deterministicFixtureReopened = reopenedCounts.constructs >= largeModelTarget.constructs
      && reopenedCounts.indicators >= largeModelTarget.indicators;

    const metrics = { fixtureRenderMs, selectionMs, dragMs, panMs, zoomMs, workspaceRoundTripMs, preflightMs, fixtureReopenMs };
    const interactions = { selectionChanged, dragChanged, panChanged, zoomChanged, modelRetainedAfterWorkspaceRoundTrip: retainedCounts.constructs >= largeModelTarget.constructs && retainedCounts.indicators >= largeModelTarget.indicators, preflightPresent, deterministicFixtureReopened };
    evidence.checks.largeModel = {
      supported: true,
      requestedFixture: "large",
      target: largeModelTarget,
      observed: counts,
      retainedAfterWorkspaceRoundTrip: retainedCounts,
      reopenedFixtureCounts: reopenedCounts,
      reopenScope: "Deterministic smoke-fixture reload only; browser preview does not prove saved-project archive reopen.",
      preflight,
      metrics,
      budgetsMs: interactionBudgetsMs,
      interactions,
    };

    for (const [name, changed] of Object.entries(interactions)) {
      if (!changed) recordFailure("large-model-interaction", `The ${name} check failed for the 20-construct/80-indicator model.`, evidence.checks.largeModel);
    }
    for (const [name, duration] of Object.entries(metrics)) {
      const budgetName = name === "fixtureRenderMs" ? "fixtureRender" : name.replace(/Ms$/, "");
      const budget = interactionBudgetsMs[budgetName];
      if (typeof duration === "number" && typeof budget === "number" && duration > budget) {
        recordFailure("large-model-performance-budget", `${name} took ${duration} ms, exceeding the ${budget} ms acceptance budget.`, evidence.checks.largeModel);
      }
    }
    await capture(page, "large-model-20c-80i", 11, viewport);
  } finally {
    evidence.consoleErrors.push(...pageErrors);
    await page.close();
  }
}
async function finalizeCoverage() {
  const requiredStates = ["launcher", "workspace-explorer", "workspace-explorer-context-menu", "workspace-explorer-rename-dialog", "data", "recode-dialog", "import-data-dialog", "model", "moderating-effect-dialog", "higher-order-dialog", "calculation-dialog", "plsc-dialog", "wpls-dialog", "gsca-dialog", "cca-dialog", "cbsem-dialog", "structural-path-randomization-dialog", "prediction-dialog", "mga-dialog", "pca-standalone-dialog", "ols-standalone-dialog", "logistic-standalone-dialog", "regression-bootstrap-ols-dialog", "regression-bootstrap-logistic-dialog", "process-v2-dialog", "empty-results", "completed-results", "mediation-results", "export-dialog"];
  const requiredCompactStates = [{ viewport: "1024x700", state: "mediation-bootstrap-inference" }];
  const expectedMatrix = [
    ...viewports.flatMap((viewport) => requiredStates.map((state) => ({ viewport: viewport.id, state }))),
    ...requiredCompactStates,
  ];
  const missingMatrix = expectedMatrix.filter((expected) => !evidence.screenshots.some((screenshot) => (
    screenshot.viewport === expected.viewport && screenshot.state === expected.state
  )));
  const missingFiles = [];
  const screenshotIntegrityErrors = [];
  const observedScreenshotPaths = new Set();
  for (const screenshot of evidence.screenshots) {
    const exactKeys = JSON.stringify(Object.keys(screenshot).sort())
      === JSON.stringify(["path", "sha256", "size", "state", "viewport"]);
    const exactPathIdentity = typeof screenshot.path === "string"
      && screenshot.path.startsWith(screenshotPathPrefix)
      && screenshot.path.endsWith(`-${screenshot.state}-${screenshot.viewport}.png`)
      && path.resolve(ROOT, screenshot.path).startsWith(`${screenshotDir}${path.sep}`);
    const exactDescriptorTypes = Number.isInteger(screenshot.size) && screenshot.size > 0
      && typeof screenshot.sha256 === "string" && /^[0-9a-f]{64}$/.test(screenshot.sha256)
      && screenshotViewportIds.has(screenshot.viewport)
      && typeof screenshot.state === "string" && /^[a-z0-9][a-z0-9-]*$/.test(screenshot.state);
    const duplicatePath = observedScreenshotPaths.has(screenshot.path);
    observedScreenshotPaths.add(screenshot.path);
    try {
      const absolutePath = path.resolve(ROOT, screenshot.path);
      const [stat, bytes] = await Promise.all([fs.stat(absolutePath), fs.readFile(absolutePath)]);
      const actualSha256 = createHash("sha256").update(bytes).digest("hex");
      if (!stat.isFile() || stat.size === 0) missingFiles.push(screenshot.path);
      if (!exactKeys || !exactPathIdentity || !exactDescriptorTypes || duplicatePath
        || stat.size !== screenshot.size || bytes.byteLength !== screenshot.size
        || actualSha256 !== screenshot.sha256) {
        screenshotIntegrityErrors.push({
          path: screenshot.path,
          exactKeys,
          exactPathIdentity,
          exactDescriptorTypes,
          duplicatePath,
          reportedSize: screenshot.size,
          actualSize: stat.size,
          reportedSha256: screenshot.sha256,
          actualSha256,
        });
      }
    } catch {
      missingFiles.push(screenshot.path);
      screenshotIntegrityErrors.push({ path: screenshot.path, unreadable: true });
    }
  }
  const runningScreenshots = evidence.screenshots.filter((screenshot) => screenshot.state === "running-calculation");
  const runningSkips = evidence.skipped.filter((skip) => skip.state === "running-calculation" || skip.id === "running-calculation");
  evidence.coverage = {
    requiredStates,
    requiredCompactStates,
    requiredViewportIds: viewports.map((viewport) => viewport.id),
    expectedRegularScreenshotCount: expectedMatrix.length,
    capturedRegularScreenshotCount: evidence.screenshots.filter((screenshot) => expectedMatrix.some((expected) => (
      expected.viewport === screenshot.viewport && expected.state === screenshot.state
    ))).length,
    missingMatrix,
    missingFiles,
    screenshotIntegrity: {
      exactDescriptorKeys: ["path", "size", "sha256", "viewport", "state"],
      pathPrefix: screenshotPathPrefix,
      descriptorCount: evidence.screenshots.length,
      uniquePathCount: observedScreenshotPaths.size,
      errors: screenshotIntegrityErrors,
      passed: screenshotIntegrityErrors.length === 0,
    },
    runningCapture: runningScreenshots.length > 0
      ? { captured: true, count: runningScreenshots.length, source: "smoke API produced a visible running lifecycle state" }
      : { captured: false, explicitSkips: runningSkips.length, nativeFollowUpRequired: true },
    artifacts: { resultPath, screenshotDir },
  };
  if (missingMatrix.length > 0) recordFailure("screenshot-matrix-incomplete", `${missingMatrix.length} required viewport/state screenshot(s) are missing.`, { missingMatrix });
  if (missingFiles.length > 0) recordFailure("screenshot-file-missing", `${missingFiles.length} screenshot artifact(s) are missing or empty.`, { missingFiles });
  if (screenshotIntegrityErrors.length > 0) recordFailure("screenshot-integrity", `${screenshotIntegrityErrors.length} screenshot descriptor(s) failed exact path, size, digest, viewport/state, or uniqueness validation.`, { screenshotIntegrityErrors });
  if (runningScreenshots.length === 0 && runningSkips.length !== viewports.length) {
    recordFailure("running-capture-not-accounted", "Running calculation evidence was neither truthfully captured nor explicitly skipped for every viewport.", { runningSkips: runningSkips.length, expected: viewports.length });
  }
}
const preview = startPreview(port);
let browser;

try {
  await ensureDir(screenshotDir);
  await auditProductionBundle();
  recordSkip("packaged-tauri-window-chrome", "Chromium preview cannot verify the number of Windows title bars, native window controls, file dialogs, or IPC.", {
    requiredNativeFollowUp: "Run the packaged Tauri visual QA and capture the real desktop window.",
  });

  await waitForPreview(baseUrl, preview.logs);
  browser = await chromium.launch({ headless: true });
  for (const viewport of viewports) {
    await exerciseViewport(browser, viewport);
  }
  await exercise200PercentScale(browser);
  await exerciseLargeModelFixture(browser);
} catch (error) {
  recordFailure("harness-exception", String(error?.stack ?? error));
} finally {
  await finalizeCoverage();
  if (browser) await browser.close();
  stopPreview(preview.server, port);
  evidence.passed = evidence.failures.length === 0 && evidence.consoleErrors.length === 0;
  await writeJson(resultPath, evidence);
}

if (!evidence.passed) {
  console.error(JSON.stringify({ failures: evidence.failures, consoleErrors: evidence.consoleErrors }, null, 2));
  process.exit(1);
}

console.log(JSON.stringify(evidence, null, 2));
