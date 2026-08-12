import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v230", "command-bar");
const OUTPUT = path.join(RESULTS, "v230_command_bar_smoke.json");
const PORT = 53230;
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
    const cluster = document.querySelector(".command-run-cluster");
    const run = document.querySelector(".run-button");
    const chip = document.querySelector(".command-blocker-chip");
    const note = document.querySelector(".workspace-coach-destination");
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      destinationContext: window.__QUICKPLS_SMOKE__?.getWorkflowDestinationContext?.() ?? null,
      commandContext: window.__QUICKPLS_SMOKE__?.getWorkflowCommandContext?.() ?? null,
      version: document.querySelector(".alpha-mark")?.textContent?.trim() ?? "",
      clusterState: cluster?.getAttribute("data-command-bar-state") ?? "",
      clusterBlockerId: cluster?.getAttribute("data-run-blocker-id") ?? "",
      clusterBlockerView: cluster?.getAttribute("data-run-blocker-view") ?? "",
      clusterMethod: cluster?.getAttribute("data-run-method") ?? "",
      runState: run?.getAttribute("data-run-state") ?? "",
      runDisabled: run instanceof HTMLButtonElement ? run.disabled : null,
      runReason: run?.getAttribute("data-run-disabled-reason") ?? "",
      runBlockerId: run?.getAttribute("data-run-blocker-id") ?? "",
      runBlockerAction: run?.getAttribute("data-run-blocker-action") ?? "",
      runDescribedBy: run?.getAttribute("aria-describedby") ?? "",
      chipText: chip?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      chipAria: chip?.getAttribute("aria-label") ?? "",
      chipTitle: chip?.getAttribute("title") ?? "",
      chipBlockerId: chip?.getAttribute("data-run-blocker-id") ?? "",
      chipBlockerView: chip?.getAttribute("data-run-blocker-view") ?? "",
      chipBlockerAction: chip?.getAttribute("data-run-blocker-action") ?? "",
      destinationNote: note?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      destinationCoach: note?.getAttribute("data-destination-coach") ?? "",
      dismissCount: document.querySelectorAll(".workspace-coach-feedback-dismiss").length,
      body: document.body.innerText,
    };
  });
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
  await page.screenshot({ path: path.join(ARTIFACTS, "01_blocked_command_bar.png"), fullPage: true });

  const initial = await snapshot(page);
  const metadataPassed = initial.version === "v2.3.0 command bar readiness"
    && initial.clusterState === "blocked"
    && initial.runState === "blocked"
    && initial.runDisabled === true
    && initial.clusterMethod === "pls_pm"
    && initial.clusterBlockerId === "runtime"
    && initial.clusterBlockerView === "models"
    && initial.runBlockerId === "runtime"
    && initial.runBlockerAction === "Open model"
    && initial.runDescribedBy === "run-disabled-reason"
    && initial.chipBlockerId === "runtime"
    && initial.chipBlockerView === "models"
    && initial.chipBlockerAction === "Open model"
    && initial.chipText.includes("Run disabled")
    && initial.chipAria.includes("Analysis runs require the offline QuickPLS desktop runtime")
    && initial.chipTitle.includes("Analysis runs require the offline QuickPLS desktop runtime");
  captures.push({ step: "blocked command bar metadata", passed: metadataPassed, initial });
  if (!metadataPassed) {
    issues.push(issue("metadata", "Command bar blocked-state metadata is incomplete", "The top bar should expose exact run state, blocker id/action/view, and disabled reason."));
  }

  await page.locator(".command-blocker-chip").click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(ARTIFACTS, "02_blocker_destination_context.png"), fullPage: true });
  const destination = await snapshot(page);
  const destinationPassed = destination.view === "models"
    && destination.destinationContext?.from === "welcome"
    && destination.destinationContext?.to === "models"
    && destination.destinationContext?.actionLabel === "Run blocker: Runtime"
    && destination.destinationContext?.coachId === "top-command-bar"
    && destination.destinationNote === "Opened Model from Run blocker: Runtime."
    && destination.destinationCoach === "top-command-bar"
    && destination.dismissCount > 0;
  captures.push({ step: "blocker destination context", passed: destinationPassed, destination });
  if (!destinationPassed) {
    issues.push(issue("destination", "Command bar blocker did not create expected destination context", "Clicking the run blocker should navigate to the target workspace and render a contextual landing note."));
  }

  if (/RÃƒÆ’Ã†â€™|RÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²|identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(destination.body)) {
    issues.push(issue("rendered-text", "Invalid rendered text", "Rendered text contains mojibake or unsupported SmartPLS equivalence wording."));
  }

  const checklist = {
    command_bar_blocked_metadata_visible: metadataPassed,
    blocker_chip_navigates_with_destination_context: destinationPassed,
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.3.0 global command bar readiness smoke",
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
