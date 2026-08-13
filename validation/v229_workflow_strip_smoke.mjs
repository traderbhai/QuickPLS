import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v229", "workflow-strip");
const OUTPUT = path.join(RESULTS, "v229_workflow_strip_smoke.json");
const PORT = 53229;
const URL = `http://127.0.0.1:${PORT}/`;

await fs.mkdir(ARTIFACTS, { recursive: true });

const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`], {
  cwd: ROOT,
  windowsHide: true,
  stdio: ["ignore", "pipe", "pipe"],
});

let logs = "";
server.stdout.on("data", (data) => { logs += data.toString(); });
server.stderr.on("data", (data) => { logs += data.toString(); });

async function waitForUrl() {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(URL, { signal: AbortSignal.timeout(1500) });
      if (response.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Timed out waiting for Vite preview. ${logs.slice(-1200)}`);
}

async function snapshot(page) {
  return page.evaluate(() => {
    const steps = Array.from(document.querySelectorAll(".workflow-step")).map((button, index) => ({
      index,
      label: button.getAttribute("data-workflow-label") ?? "",
      view: button.getAttribute("data-workflow-view") ?? "",
      state: button.getAttribute("data-workflow-state") ?? "",
      action: button.getAttribute("data-workflow-action") ?? "",
      detail: button.getAttribute("data-workflow-detail") ?? "",
      ariaLabel: button.getAttribute("aria-label") ?? "",
      current: button.getAttribute("aria-current") ?? "",
    }));
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      destinationContext: window.__QUICKPLS_SMOKE__?.getWorkflowDestinationContext?.() ?? null,
      commandContext: window.__QUICKPLS_SMOKE__?.getWorkflowCommandContext?.() ?? null,
      destinationNote: document.querySelector(".workspace-coach-destination")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      dismissCount: document.querySelectorAll(".workspace-coach-feedback-dismiss").length,
      steps,
      body: document.body.innerText,
    };
  });
}

async function clickStep(page, view) {
  await page.evaluate((targetView) => {
    const button = Array.from(document.querySelectorAll(".workflow-step")).find((step) => step.getAttribute("data-workflow-view") === targetView);
    if (button instanceof HTMLButtonElement) button.click();
  }, view);
  await page.waitForTimeout(500);
}

async function clickDismiss(page) {
  await page.locator(".workspace-coach-feedback-dismiss").first().click();
  await page.waitForTimeout(250);
}

function issue(id, title, detail) {
  return { id, severity: "high", title, detail };
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  const issues = [];
  const captures = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });

  await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__?.loadEmptyProject && window.__QUICKPLS_SMOKE__?.getWorkflowDestinationContext), null, { timeout: 10_000 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
  await page.waitForTimeout(300);
  await page.screenshot({ path: path.join(ARTIFACTS, "01_initial_workflow_strip.png"), fullPage: true });

  const initial = await snapshot(page);
  if (!initial.body.includes("v2.2.9 workflow strip alignment")) {
    issues.push(issue("version-label", "Version label mismatch", "Rendered app should show v2.2.9 workflow strip alignment."));
  }
  const expectedViews = ["data", "models", "analyses", "run", "runs", "reports"];
  const metadataComplete = expectedViews.every((view) => {
    const step = initial.steps.find((candidate) => candidate.view === view);
    return Boolean(step?.label && step?.state && step?.action && step?.detail && step?.ariaLabel);
  });
  captures.push({ step: "metadata complete", passed: metadataComplete, steps: initial.steps });
  if (!metadataComplete) issues.push(issue("metadata", "Workflow strip metadata incomplete", "Every workflow step should expose view, label, action, detail, state, and aria label."));

  await clickStep(page, "data");
  const dataDestination = await snapshot(page);
  await page.screenshot({ path: path.join(ARTIFACTS, "02_data_destination_context.png"), fullPage: true });
  const dataContextPassed = dataDestination.view === "data"
    && dataDestination.destinationContext?.from === "welcome"
    && dataDestination.destinationContext?.to === "data"
    && dataDestination.destinationContext?.actionLabel === "Workflow: Data"
    && dataDestination.destinationContext?.coachId === "workflow-strip"
    && dataDestination.destinationNote === "Opened Data from Workflow: Data."
    && dataDestination.dismissCount > 0;
  captures.push({ step: "data destination context", passed: dataContextPassed, dataDestination });
  if (!dataContextPassed) issues.push(issue("data-destination", "Data workflow step did not create expected destination context", "Clicking Data should identify the workflow strip as the source."));

  await clickDismiss(page);
  const dismissed = await snapshot(page);
  const dismissPassed = !dismissed.destinationContext && !dismissed.commandContext && dismissed.dismissCount === 0;
  captures.push({ step: "destination dismissed", passed: dismissPassed, dismissed });
  if (!dismissPassed) issues.push(issue("dismiss", "Workflow strip destination feedback did not dismiss", "Dismiss should clear workflow feedback."));

  await clickStep(page, "data");
  const sameStep = await snapshot(page);
  await page.screenshot({ path: path.join(ARTIFACTS, "03_same_step_no_destination_context.png"), fullPage: true });
  const sameStepPassed = sameStep.view === "data" && !sameStep.destinationContext && !sameStep.destinationNote && sameStep.dismissCount === 0;
  captures.push({ step: "same step no context", passed: sameStepPassed, sameStep });
  if (!sameStepPassed) issues.push(issue("same-step", "Current workflow step created redundant feedback", "Clicking the current workflow step should not create a new destination note."));

  await clickStep(page, "analyses");
  const setupDestination = await snapshot(page);
  await page.screenshot({ path: path.join(ARTIFACTS, "04_setup_destination_context.png"), fullPage: true });
  const setupContextPassed = setupDestination.view === "analyses"
    && setupDestination.destinationContext?.from === "data"
    && setupDestination.destinationContext?.to === "analyses"
    && setupDestination.destinationContext?.actionLabel === "Workflow: Setup"
    && setupDestination.destinationContext?.coachId === "workflow-strip"
    && setupDestination.destinationNote === "Opened Setup from Workflow: Setup.";
  captures.push({ step: "setup destination context", passed: setupContextPassed, setupDestination });
  if (!setupContextPassed) issues.push(issue("setup-destination", "Setup workflow step did not create expected destination context", "Clicking Setup should preserve source and target metadata."));

  if (/RÃƒÆ’|RÃƒâ€šÃ‚Â²|identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(setupDestination.body)) {
    issues.push(issue("rendered-text", "Invalid rendered text", "Rendered text contains mojibake or unsupported SmartPLS equivalence wording."));
  }

  const checklist = {
    workflow_metadata_complete: metadataComplete,
    workflow_click_records_destination_context: dataContextPassed && setupContextPassed,
    workflow_feedback_dismisses: dismissPassed,
    current_step_click_does_not_create_feedback: sameStepPassed,
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.9 workflow strip context alignment smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    captures,
    issues,
    errors,
    screenshots_dir: ARTIFACTS,
  };
  await fs.writeFile(OUTPUT, JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  if (!result.passed) process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (server.pid) {
    try {
      execFileSync("taskkill.exe", ["/PID", String(server.pid), "/T", "/F"], { stdio: "ignore" });
    } catch {
      server.kill();
    }
    try {
      execFileSync("powershell.exe", ["-NoProfile", "-Command", `(Get-NetTCPConnection -LocalPort ${PORT} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess -Unique) | ForEach-Object { Stop-Process -Id $_ -Force }`], { stdio: "ignore" });
    } catch {
      // Best-effort cleanup for detached Vite children.
    }
  } else {
    server.kill();
  }
}
