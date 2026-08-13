import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v227", "command-feedback");
const OUTPUT = path.join(RESULTS, "v227_command_feedback_smoke.json");
const PORT = 53227;
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
  await page.evaluate(() => {
    document.querySelector(".page-host")?.scrollTo({ top: 0, left: 0 });
    window.scrollTo({ top: 0, left: 0 });
  });
}

async function snapshot(page) {
  return page.evaluate(() => {
    const coach = document.querySelector(".workspace-coach");
    const command = document.querySelector(".workspace-coach-command");
    const actions = Array.from(coach?.querySelectorAll(".workspace-coach-actions button") ?? []).map((button, index) => ({
      index,
      label: button.getAttribute("data-action-label") ?? "",
      view: button.getAttribute("data-action-view") ?? "",
      event: button.getAttribute("data-action-event") ?? "",
      disabledData: button.getAttribute("data-action-disabled") ?? "",
      disabled: button.hasAttribute("disabled"),
      reasonId: button.getAttribute("aria-describedby") ?? "",
    }));
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      context: window.__QUICKPLS_SMOKE__?.getWorkflowCommandContext?.() ?? null,
      destinationContext: window.__QUICKPLS_SMOKE__?.getWorkflowDestinationContext?.() ?? null,
      coachId: coach?.getAttribute("data-coach-id") ?? "",
      actions,
      command: command ? {
        text: command.textContent?.replace(/\s+/g, " ").trim() ?? "",
        from: command.getAttribute("data-command-from") ?? "",
        event: command.getAttribute("data-command-event") ?? "",
        action: command.getAttribute("data-command-action") ?? "",
        coach: command.getAttribute("data-command-coach") ?? "",
      } : null,
      body: document.body.innerText,
    };
  });
}

async function clickAction(page, index) {
  await page.evaluate((buttonIndex) => {
    const button = document.querySelectorAll(".workspace-coach-actions button")[buttonIndex];
    if (button instanceof HTMLButtonElement) button.click();
  }, index);
  await page.waitForTimeout(500);
}

function issue(id, severity, title, detail) {
  return { id, severity, title, detail };
}

let browser;
try {
  await waitForUrl();
  browser = await chromium.launch({ headless: true });
  const errors = [];
  const issues = [];
  const captures = [];
  const viewports = [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1280x800", width: 1280, height: 800 },
  ];
  const views = ["welcome", "data", "models", "analyses", "run", "runs", "reports"];

  for (const viewport of viewports) {
    const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height }, deviceScaleFactor: 1 });
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__?.getWorkflowCommandContext && window.__QUICKPLS_SMOKE__?.loadEmptyProject), null, { timeout: 10_000 });
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
    await page.waitForTimeout(250);

    for (const view of views) {
      await setView(page, view);
      const before = await snapshot(page);
      await page.screenshot({ path: path.join(ARTIFACTS, `${viewport.name}_${view}_before.png`), fullPage: true });
      if (!before.body.includes("v2.2.7 command feedback")) {
        issues.push(issue(`version-${viewport.name}-${view}`, "high", "Version label mismatch", "Rendered app should show v2.2.7 command feedback."));
      }
      if (/RÃƒ|RÃ‚Â²|identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(before.body)) {
        issues.push(issue(`source-text-${viewport.name}-${view}`, "high", "Invalid rendered text", "Rendered text contains mojibake or unsupported SmartPLS equivalence wording."));
      }

      for (const action of before.actions.filter((item) => !item.disabled && item.event)) {
        await setView(page, view);
        const baseline = await snapshot(page);
        await clickAction(page, action.index);
        const after = await snapshot(page);
        await page.screenshot({ path: path.join(ARTIFACTS, `${viewport.name}_${view}_${action.index}_command_after.png`), fullPage: true });
        const passed = after.context?.from === view
          && after.context?.event === action.event
          && after.context?.actionLabel === action.label
          && after.context?.coachId === before.coachId
          && after.command?.from === view
          && after.command?.event === action.event
          && after.command?.action === action.label
          && after.command?.coach === before.coachId
          && after.command?.text?.includes(action.label);
        captures.push({ viewport: viewport.name, sourceView: view, coachId: before.coachId, action, contextBefore: baseline.context, context: after.context, command: after.command, passed });
        if (!passed) {
          issues.push(issue(`command-${viewport.name}-${view}-${action.index}`, "high", "Command feedback mismatch", `${action.label} should write command context for ${action.event}.`));
        }
      }

      for (const action of before.actions.filter((item) => item.disabled && item.event)) {
        await setView(page, view);
        const disabledBaseline = await snapshot(page);
        await clickAction(page, action.index);
        const after = await snapshot(page);
        const passed = after.view === view && JSON.stringify(after.context) === JSON.stringify(disabledBaseline.context);
        captures.push({ viewport: viewport.name, sourceView: view, coachId: before.coachId, action, disabled: true, afterView: after.view, context: after.context, passed });
        if (!passed) {
          issues.push(issue(`disabled-command-${viewport.name}-${view}-${action.index}`, "high", "Disabled command changed context", `${action.label} should not write command feedback.`));
        }
      }
    }
    await page.close();
  }

  const enabledCommandCaptures = captures.filter((item) => !item.disabled);
  const disabledCommandCaptures = captures.filter((item) => item.disabled);
  if (enabledCommandCaptures.length < 2) issues.push(issue("enabled-command-coverage", "medium", "Insufficient enabled command coverage", `Expected at least 2 enabled command actions, observed ${enabledCommandCaptures.length}.`));
  if (disabledCommandCaptures.length < 1) issues.push(issue("disabled-command-coverage", "medium", "No disabled command covered", "Expected at least one disabled command action coverage item."));

  const checklist = {
    command_context_created: enabledCommandCaptures.length >= 2 && enabledCommandCaptures.every((item) => item.passed),
    command_note_rendered: enabledCommandCaptures.every((item) => item.command?.text?.includes(item.action.label)),
    disabled_commands_do_not_change_context: disabledCommandCaptures.length >= 1 && disabledCommandCaptures.every((item) => item.passed),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.7 workflow command feedback smoke",
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
