import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v224", "coach-actions");
const OUTPUT = path.join(RESULTS, "v224_coach_actions_smoke.json");
const PORT = 53224;
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
  await page.waitForTimeout(300);
  await page.evaluate(() => {
    document.querySelector(".page-host")?.scrollTo({ top: 0, left: 0 });
    window.scrollTo({ top: 0, left: 0 });
  });
}

async function coachSnapshot(page) {
  return page.evaluate(() => {
    const coach = document.querySelector(".workspace-coach");
    const actionBlocks = Array.from(coach?.querySelectorAll(".workspace-coach-action-block") ?? []);
    const actions = actionBlocks.map((block) => {
      const button = block.querySelector("button");
      const reasonId = button?.getAttribute("aria-describedby") ?? "";
      const reason = reasonId ? document.getElementById(reasonId)?.textContent?.replace(/\s+/g, " ").trim() ?? "" : "";
      return {
        text: button?.textContent?.replace(/\s+/g, " ").trim() ?? "",
        label: button?.getAttribute("data-action-label") ?? "",
        disabledData: button?.getAttribute("data-action-disabled") ?? "",
        disabled: Boolean(button?.hasAttribute("disabled")),
        title: button?.getAttribute("title") ?? "",
        ariaDescribedBy: reasonId,
        reason,
      };
    });
    const body = document.body.innerText;
    return {
      present: Boolean(coach),
      id: coach?.getAttribute("data-coach-id") ?? "",
      statusClass: coach?.className ?? "",
      actions,
      actionCount: actions.length,
      disabledCount: actions.filter((action) => action.disabled).length,
      disabledReasonCount: actions.filter((action) => action.disabled && action.reason).length,
      duplicateActionKeys: actions
        .map((action) => `${action.label}|${action.disabledData}|${action.title}`)
        .filter((key, index, keys) => keys.indexOf(key) !== index),
      versionVisible: body.includes("v2.2.4 coach actions"),
      body,
    };
  });
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
    await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

    for (const view of views) {
      await setView(page, view);
      const snapshot = await coachSnapshot(page);
      await page.screenshot({ path: path.join(ARTIFACTS, `${viewport.name}_${view}.png`), fullPage: true });
      captures.push({ viewport: viewport.name, view, ...snapshot, body: undefined });

      if (!snapshot.present) issues.push(issue(`coach-missing-${viewport.name}-${view}`, "high", "Workflow coach missing", `No workflow coach rendered for ${view} at ${viewport.name}.`));
      if (!snapshot.id) issues.push(issue(`coach-id-${viewport.name}-${view}`, "high", "Coach id missing", "The coach must expose data-coach-id for deterministic smoke checks."));
      if (snapshot.actionCount < 1) issues.push(issue(`actions-missing-${viewport.name}-${view}`, "high", "Coach action missing", "Every coach state must expose at least one action."));
      if (!snapshot.actions.every((action) => action.label && action.disabledData)) issues.push(issue(`action-metadata-${viewport.name}-${view}`, "high", "Action metadata missing", "Every coach action must expose data-action-label and data-action-disabled."));
      if (snapshot.actions.some((action) => action.disabled && (!action.ariaDescribedBy || !action.reason))) {
        issues.push(issue(`disabled-reason-${viewport.name}-${view}`, "high", "Disabled action reason missing", "Disabled coach actions must expose a visible reason and aria-describedby."));
      }
      if (snapshot.duplicateActionKeys.length > 0) issues.push(issue(`duplicate-actions-${viewport.name}-${view}`, "medium", "Duplicate coach actions visible", `Duplicate actions: ${snapshot.duplicateActionKeys.join(", ")}`));
      if (!snapshot.versionVisible) issues.push(issue(`version-${viewport.name}-${view}`, "high", "Version label mismatch", "The rendered app should show v2.2.4 coach actions."));
      if (/RÃƒÆ’Ã†â€™|RÃƒÆ’Ã¢â‚¬Å¡|ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²/.test(snapshot.body)) issues.push(issue(`mojibake-${viewport.name}-${view}`, "high", "Mojibake visible", "Rendered text includes corrupted encoding artifacts."));
      if (/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(snapshot.body)) issues.push(issue(`smartpls-claim-${viewport.name}-${view}`, "high", "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
    }
    await page.close();
  }

  const disabledReasonTotal = captures.reduce((total, capture) => total + capture.disabledReasonCount, 0);
  if (disabledReasonTotal < 1) issues.push(issue("disabled-reason-observed", "high", "No disabled reason observed", "Smoke should observe at least one disabled coach action with a visible reason."));

  const checklist = {
    coach_rendered_all_views: captures.every((capture) => capture.present),
    action_metadata_present: captures.every((capture) => capture.actions.every((action) => action.label && action.disabledData)),
    disabled_reasons_visible: disabledReasonTotal > 0 && captures.every((capture) => capture.actions.every((action) => !action.disabled || (action.ariaDescribedBy && action.reason))),
    no_duplicate_actions: captures.every((capture) => capture.duplicateActionKeys.length === 0),
    version_visible: captures.every((capture) => capture.versionVisible),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.4 workflow coach action clarity smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    disabled_reason_total: disabledReasonTotal,
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
