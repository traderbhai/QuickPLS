import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v223", "model-workflow");
const OUTPUT = path.join(RESULTS, "v223_model_workflow_smoke.json");
const PORT = 53223;
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
  await page.waitForTimeout(350);
}

async function modelSnapshot(page) {
  return page.evaluate(() => {
    const band = document.querySelector(".model-workflow-band");
    const strip = band?.querySelector(".workflow-strip");
    const coach = band?.querySelector(".workspace-coach");
    const steps = Array.from(band?.querySelectorAll(".workflow-step") ?? []).map((step) => ({
      text: step.textContent?.replace(/\s+/g, " ").trim() ?? "",
      state: step.getAttribute("data-workflow-state") ?? "",
      title: step.getAttribute("title") ?? "",
      aria: step.getAttribute("aria-label") ?? "",
      current: step.getAttribute("aria-current") ?? "",
    }));
    const body = document.body.innerText;
    return {
      bandPresent: Boolean(band),
      stripPresent: Boolean(strip),
      coachPresent: Boolean(coach),
      coachId: coach?.getAttribute("data-coach-id") ?? "",
      stepCount: steps.length,
      steps,
      workflowOverflow: strip ? strip.scrollWidth - strip.clientWidth : 9999,
      explorerPresent: Boolean(document.querySelector(".explorer")),
      canvasPresent: Boolean(document.querySelector(".model-canvas")),
      reactFlowPresent: Boolean(document.querySelector(".model-canvas .react-flow")),
      toolbarPresent: Boolean(document.querySelector(".canvas-toolbar")),
      inspectorPresent: Boolean(document.querySelector(".inspector")),
      modelIsCurrent: steps.some((step) => step.text.toLowerCase().includes("model") && step.current === "step" && step.state === "current"),
      versionVisible: body.includes("v2.2.3 model workflow"),
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

  for (const viewport of viewports) {
    const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height }, deviceScaleFactor: 1 });
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${URL}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });
    await setView(page, "models");
    const snapshot = await modelSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, `${viewport.name}_model.png`), fullPage: true });
    captures.push({ viewport: viewport.name, ...snapshot, body: undefined });

    if (!snapshot.bandPresent) issues.push(issue(`band-missing-${viewport.name}`, "high", "Model workflow band missing", "The Model workspace must render model-workflow-band."));
    if (!snapshot.stripPresent) issues.push(issue(`strip-missing-${viewport.name}`, "high", "Workflow strip missing", "The Model workspace must render WorkflowStrip inside the model workflow band."));
    if (!snapshot.coachPresent) issues.push(issue(`coach-missing-${viewport.name}`, "high", "Workspace coach missing", "The Model workspace must render WorkspaceCoach inside the model workflow band."));
    if (!snapshot.coachId.startsWith("model-")) issues.push(issue(`coach-id-${viewport.name}`, "high", "Model-specific coach missing", `Expected model-specific coach id, found ${snapshot.coachId || "none"}.`));
    if (snapshot.stepCount !== 6) issues.push(issue(`step-count-${viewport.name}`, "high", "Unexpected workflow step count", `Expected 6 steps, found ${snapshot.stepCount}.`));
    if (!snapshot.modelIsCurrent) issues.push(issue(`model-current-${viewport.name}`, "high", "Model step is not current", "The workflow strip must mark Model as the current step in the Model workspace."));
    if (snapshot.workflowOverflow > 2) issues.push(issue(`workflow-overflow-${viewport.name}`, "medium", "Model workflow strip overflows", `Overflow is ${snapshot.workflowOverflow}px.`));
    if (!snapshot.explorerPresent || !snapshot.canvasPresent || !snapshot.reactFlowPresent || !snapshot.toolbarPresent || !snapshot.inspectorPresent) {
      issues.push(issue(`designer-shell-${viewport.name}`, "high", "Designer shell incomplete", "Explorer, ModelCanvas, React Flow canvas, toolbar, and Inspector must remain present."));
    }
    if (!snapshot.versionVisible) issues.push(issue(`version-${viewport.name}`, "high", "Version label mismatch", "The rendered app should show v2.2.3 model workflow."));
    if (/RÃƒÆ’Ã†â€™|RÃƒÆ’Ã¢â‚¬Å¡|ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²/.test(snapshot.body)) issues.push(issue(`mojibake-${viewport.name}`, "high", "Mojibake visible", "Rendered text includes corrupted encoding artifacts."));
    if (/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(snapshot.body)) issues.push(issue(`smartpls-claim-${viewport.name}`, "high", "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
    await page.close();
  }

  const checklist = {
    model_workflow_band_rendered: captures.every((capture) => capture.bandPresent),
    workflow_strip_rendered: captures.every((capture) => capture.stripPresent && capture.stepCount === 6 && capture.modelIsCurrent),
    model_coach_rendered: captures.every((capture) => capture.coachPresent && capture.coachId.startsWith("model-")),
    designer_shell_preserved: captures.every((capture) => capture.explorerPresent && capture.canvasPresent && capture.reactFlowPresent && capture.toolbarPresent && capture.inspectorPresent),
    desktop_width_no_overflow: captures.every((capture) => capture.workflowOverflow <= 2),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.3 model workflow context smoke",
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
