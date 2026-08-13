import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v222", "workflow-step");
const OUTPUT = path.join(RESULTS, "v222_workflow_step_smoke.json");
const PORT = 53222;
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

async function workflowSnapshot(page) {
  return page.evaluate(() => {
    const strip = document.querySelector(".workflow-strip");
    const steps = Array.from(document.querySelectorAll(".workflow-step")).map((step) => ({
      text: step.textContent?.replace(/\s+/g, " ").trim() ?? "",
      state: step.getAttribute("data-workflow-state") ?? "",
      title: step.getAttribute("title") ?? "",
      aria: step.getAttribute("aria-label") ?? "",
      current: step.getAttribute("aria-current") ?? "",
    }));
    const rect = strip?.getBoundingClientRect();
    return {
      present: Boolean(strip),
      stepCount: steps.length,
      steps,
      overflow: strip ? strip.scrollWidth - strip.clientWidth : 9999,
      height: rect?.height ?? 0,
      body: document.body.innerText,
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
  const views = ["welcome", "data", "analyses", "run", "runs", "reports"];

  for (const viewport of viewports) {
    const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height }, deviceScaleFactor: 1 });
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

    for (const view of views) {
      await setView(page, view);
      const snapshot = await workflowSnapshot(page);
      await page.screenshot({ path: path.join(ARTIFACTS, `${viewport.name}_${view}.png`), fullPage: true });
      captures.push({ viewport: viewport.name, view, ...snapshot, body: undefined });
      if (!snapshot.present) issues.push(issue(`strip-missing-${viewport.name}-${view}`, "high", "Workflow strip missing", `No workflow strip rendered for ${view} at ${viewport.name}.`));
      if (snapshot.stepCount !== 6) issues.push(issue(`step-count-${viewport.name}-${view}`, "high", "Unexpected workflow step count", `Expected 6 steps, found ${snapshot.stepCount}.`));
      if (snapshot.overflow > 2) issues.push(issue(`overflow-${viewport.name}-${view}`, "medium", "Workflow strip overflows", `Overflow is ${snapshot.overflow}px for ${view} at ${viewport.name}.`));
      if (!snapshot.steps.every((step) => step.state && step.title && step.aria)) issues.push(issue(`metadata-${viewport.name}-${view}`, "high", "Workflow step metadata missing", "Every workflow step must expose data-workflow-state, title, and aria-label."));
      if (view !== "welcome" && !snapshot.steps.some((step) => step.state === "current" && step.current === "step")) issues.push(issue(`current-${viewport.name}-${view}`, "high", "Current workflow step missing", `No current step marker for ${view} at ${viewport.name}.`));
      if (!snapshot.steps.some((step) => ["complete", "next", "blocked", "ready", "current"].includes(step.state))) issues.push(issue(`states-${viewport.name}-${view}`, "high", "Workflow states missing", "No known workflow states were rendered."));
      if (!snapshot.body.includes("v2.2.2 workflow steps")) issues.push(issue(`version-${viewport.name}-${view}`, "high", "Version label mismatch", "The rendered app should show v2.2.2 workflow steps."));
      if (/RÃƒÆ’|RÃƒâ€š|Ãƒâ€šÃ‚Â²/.test(snapshot.body)) issues.push(issue(`mojibake-${viewport.name}-${view}`, "high", "Mojibake visible", "Rendered text includes corrupted encoding artifacts."));
      if (/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(snapshot.body)) issues.push(issue(`smartpls-claim-${viewport.name}-${view}`, "high", "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
    }
    await page.close();
  }

  const observedStates = new Set(captures.flatMap((capture) => capture.steps.map((step) => step.state)));
  if (!observedStates.has("complete")) issues.push(issue("state-complete", "medium", "No complete state observed", "Smoke should observe at least one completed workflow step."));
  if (!observedStates.has("current")) issues.push(issue("state-current", "medium", "No current state observed", "Smoke should observe a current workflow step."));

  const checklist = {
    workflow_strip_rendered: captures.every((capture) => capture.present),
    six_steps_rendered: captures.every((capture) => capture.stepCount === 6),
    state_metadata_present: captures.every((capture) => capture.steps.every((step) => step.state && step.title && step.aria)),
    desktop_width_no_overflow: captures.every((capture) => capture.overflow <= 2),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.2 workflow step clarity smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    observed_states: Array.from(observedStates).sort(),
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
