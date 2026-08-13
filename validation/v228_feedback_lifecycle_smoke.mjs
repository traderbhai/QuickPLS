import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v228", "feedback-lifecycle");
const OUTPUT = path.join(RESULTS, "v228_feedback_lifecycle_smoke.json");
const PORT = 53228;
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

async function setView(page, view) {
  await page.evaluate((nextView) => window.__QUICKPLS_SMOKE__?.setView(nextView), view);
  await page.waitForTimeout(250);
}

async function snapshot(page) {
  return page.evaluate(() => {
    const actions = Array.from(document.querySelectorAll(".workspace-coach-actions button")).map((button, index) => ({
      index,
      label: button.getAttribute("data-action-label") ?? "",
      view: button.getAttribute("data-action-view") ?? "",
      event: button.getAttribute("data-action-event") ?? "",
      disabled: button.hasAttribute("disabled"),
    }));
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      destinationContext: window.__QUICKPLS_SMOKE__?.getWorkflowDestinationContext?.() ?? null,
      commandContext: window.__QUICKPLS_SMOKE__?.getWorkflowCommandContext?.() ?? null,
      destinationNote: document.querySelector(".workspace-coach-destination")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      commandNote: document.querySelector(".workspace-coach-command")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      dismissCount: document.querySelectorAll(".workspace-coach-feedback-dismiss").length,
      body: document.body.innerText,
      actions,
    };
  });
}

async function clickCoachAction(page, action) {
  await page.evaluate((buttonIndex) => {
    const button = document.querySelectorAll(".workspace-coach-actions button")[buttonIndex];
    if (button instanceof HTMLButtonElement) button.click();
  }, action.index);
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
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__?.loadEmptyProject && window.__QUICKPLS_SMOKE__?.getWorkflowCommandContext), null, { timeout: 10_000 });
  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
  await page.waitForTimeout(300);
  await page.screenshot({ path: path.join(ARTIFACTS, "01_empty_welcome.png"), fullPage: true });

  const initial = await snapshot(page);
  if (!initial.body.includes("v2.2.8 feedback lifecycle")) {
    issues.push(issue("version-label", "Version label mismatch", "Rendered app should show v2.2.8 feedback lifecycle."));
  }

  const commandAction = initial.actions.find((action) => !action.disabled && action.event);
  if (!commandAction) {
    issues.push(issue("command-action", "No enabled command action", "Expected at least one enabled command action on the empty welcome coach."));
  } else {
    await clickCoachAction(page, commandAction);
    const commandCreated = await snapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "02_command_feedback_created.png"), fullPage: true });
    const created = commandCreated.commandContext?.actionLabel === commandAction.label
      && commandCreated.commandNote.includes(commandAction.label)
      && commandCreated.dismissCount > 0;
    captures.push({ step: "command created", passed: created, commandAction, commandCreated });
    if (!created) issues.push(issue("command-created", "Command feedback was not created", `${commandAction.label} should create a dismissible command note.`));

    await clickDismiss(page);
    const commandDismissed = await snapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "03_command_feedback_dismissed.png"), fullPage: true });
    const dismissed = !commandDismissed.commandContext && !commandDismissed.destinationContext && commandDismissed.dismissCount === 0;
    captures.push({ step: "command dismissed", passed: dismissed, commandDismissed });
    if (!dismissed) issues.push(issue("command-dismissed", "Command feedback was not dismissed", "Dismiss should clear both workflow feedback contexts."));

    await clickCoachAction(page, commandAction);
    await setView(page, "analyses");
    const commandClearedByNavigation = await snapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "04_command_cleared_by_navigation.png"), fullPage: true });
    const navCleared = !commandClearedByNavigation.commandContext && !commandClearedByNavigation.commandNote;
    captures.push({ step: "command cleared by navigation", passed: navCleared, commandClearedByNavigation });
    if (!navCleared) issues.push(issue("command-navigation", "Command feedback survived ordinary navigation", "Cross-workspace navigation without coach context should clear command feedback."));
  }

  await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
  await page.waitForTimeout(300);
  const welcome = await snapshot(page);
  const destinationAction = welcome.actions.find((action) => !action.disabled && action.view && action.view !== welcome.view);
  if (!destinationAction) {
    issues.push(issue("destination-action", "No enabled destination action", "Expected at least one enabled view-target action on the empty welcome coach."));
  } else {
    await clickCoachAction(page, destinationAction);
    const destinationCreated = await snapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "05_destination_feedback_created.png"), fullPage: true });
    const created = destinationCreated.destinationContext?.actionLabel === destinationAction.label
      && destinationCreated.destinationNote.includes(destinationAction.label)
      && destinationCreated.dismissCount > 0;
    captures.push({ step: "destination created", passed: created, destinationAction, destinationCreated });
    if (!created) issues.push(issue("destination-created", "Destination feedback was not created", `${destinationAction.label} should create a dismissible destination note.`));

    await clickDismiss(page);
    const destinationDismissed = await snapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "06_destination_feedback_dismissed.png"), fullPage: true });
    const dismissed = !destinationDismissed.commandContext && !destinationDismissed.destinationContext && destinationDismissed.dismissCount === 0;
    captures.push({ step: "destination dismissed", passed: dismissed, destinationDismissed });
    if (!dismissed) issues.push(issue("destination-dismissed", "Destination feedback was not dismissed", "Dismiss should clear destination feedback."));

    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
    await page.waitForTimeout(300);
    const welcomeAgain = await snapshot(page);
    const replacementAction = welcomeAgain.actions.find((action) => !action.disabled && action.view && action.view !== welcomeAgain.view);
    if (!replacementAction) {
      issues.push(issue("replacement-action", "No replacement setup action", "Expected a view-target action before project replacement check."));
    } else {
      await clickCoachAction(page, replacementAction);
      await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
      await page.waitForTimeout(300);
      const clearedByProjectLoad = await snapshot(page);
      await page.screenshot({ path: path.join(ARTIFACTS, "07_feedback_cleared_by_project_load.png"), fullPage: true });
      const projectCleared = !clearedByProjectLoad.commandContext && !clearedByProjectLoad.destinationContext && clearedByProjectLoad.dismissCount === 0;
      captures.push({ step: "feedback cleared by project load", passed: projectCleared, clearedByProjectLoad });
      if (!projectCleared) issues.push(issue("project-load", "Feedback survived project replacement", "Project load should clear workflow feedback."));
    }
  }

  if (/RÃƒÆ’|RÃƒâ€šÃ‚Â²|identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test((await snapshot(page)).body)) {
    issues.push(issue("rendered-text", "Invalid rendered text", "Rendered text contains mojibake or unsupported SmartPLS equivalence wording."));
  }

  const checklist = {
    command_feedback_created_and_dismissed: captures.some((item) => item.step === "command dismissed" && item.passed),
    command_feedback_clears_on_navigation: captures.some((item) => item.step === "command cleared by navigation" && item.passed),
    destination_feedback_created_and_dismissed: captures.some((item) => item.step === "destination dismissed" && item.passed),
    feedback_clears_on_project_load: captures.some((item) => item.step === "feedback cleared by project load" && item.passed),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.8 workflow feedback lifecycle smoke",
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
