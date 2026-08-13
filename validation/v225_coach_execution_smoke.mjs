import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v225", "coach-execution");
const OUTPUT = path.join(RESULTS, "v225_coach_execution_smoke.json");
const PORT = 53225;
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

async function getView(page) {
  return page.evaluate(() => window.__QUICKPLS_SMOKE__?.getView() ?? "");
}

async function coachSnapshot(page) {
  return page.evaluate(() => {
    const coach = document.querySelector(".workspace-coach");
    const buttons = Array.from(coach?.querySelectorAll(".workspace-coach-actions button") ?? []);
    const actions = buttons.map((button, index) => ({
      index,
      text: button.textContent?.replace(/\s+/g, " ").trim() ?? "",
      label: button.getAttribute("data-action-label") ?? "",
      view: button.getAttribute("data-action-view") ?? "",
      event: button.getAttribute("data-action-event") ?? "",
      disabledData: button.getAttribute("data-action-disabled") ?? "",
      disabled: button.hasAttribute("disabled"),
      title: button.getAttribute("title") ?? "",
      reasonId: button.getAttribute("aria-describedby") ?? "",
    }));
    return {
      coachId: coach?.getAttribute("data-coach-id") ?? "",
      actions,
      body: document.body.innerText,
    };
  });
}

async function clickAction(page, index) {
  await page.evaluate((buttonIndex) => {
    const button = document.querySelectorAll(".workspace-coach-actions button")[buttonIndex];
    if (button instanceof HTMLButtonElement) button.click();
  }, index);
  await page.waitForTimeout(350);
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
  const clickResults = [];
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
    await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__?.getView), null, { timeout: 10_000 });

    for (const view of views) {
      await setView(page, view);
      const snapshot = await coachSnapshot(page);
      await page.screenshot({ path: path.join(ARTIFACTS, `${viewport.name}_${view}.png`), fullPage: true });
      captures.push({ viewport: viewport.name, view, coachId: snapshot.coachId, actions: snapshot.actions });

      if (!snapshot.coachId) issues.push(issue(`coach-id-${viewport.name}-${view}`, "high", "Coach id missing", "The coach must expose data-coach-id."));
      if (!snapshot.body.includes("v2.2.5 coach execution")) issues.push(issue(`version-${viewport.name}-${view}`, "high", "Version label mismatch", "The rendered app should show v2.2.5 coach execution."));
      if (/RÃƒÆ’Ã†â€™|RÃƒÆ’Ã¢â‚¬Å¡|ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²/.test(snapshot.body)) issues.push(issue(`mojibake-${viewport.name}-${view}`, "high", "Mojibake visible", "Rendered text includes corrupted encoding artifacts."));
      if (/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(snapshot.body)) issues.push(issue(`smartpls-claim-${viewport.name}-${view}`, "high", "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));
      if (!snapshot.actions.every((action) => action.label && action.disabledData && ("view" in action) && ("event" in action))) {
        issues.push(issue(`metadata-${viewport.name}-${view}`, "high", "Coach action target metadata missing", "Every coach action must expose label, disabled, target view, and target event metadata."));
      }
      if (!snapshot.actions.some((action) => action.view || action.event)) {
        issues.push(issue(`target-${viewport.name}-${view}`, "high", "Coach action target missing", "At least one coach action must expose either a view target or command event."));
      }

      for (const action of snapshot.actions.filter((item) => !item.disabled && item.view)) {
        await setView(page, view);
        await clickAction(page, action.index);
        const after = await getView(page);
        const passed = after === action.view;
        clickResults.push({ viewport: viewport.name, sourceView: view, action: action.label, expectedView: action.view, actualView: after, passed });
        if (!passed) {
          issues.push(issue(`click-${viewport.name}-${view}-${action.index}`, "high", "Coach action did not navigate", `${action.label} expected ${action.view}, got ${after}.`));
        }
      }

      for (const action of snapshot.actions.filter((item) => item.disabled)) {
        await setView(page, view);
        await clickAction(page, action.index);
        const after = await getView(page);
        const passed = after === view;
        clickResults.push({ viewport: viewport.name, sourceView: view, action: action.label, expectedView: view, actualView: after, disabled: true, passed });
        if (!passed) {
          issues.push(issue(`disabled-click-${viewport.name}-${view}-${action.index}`, "high", "Disabled coach action changed view", `${action.label} should remain on ${view}, got ${after}.`));
        }
      }
    }
    await page.close();
  }

  const viewClicks = clickResults.filter((result) => !result.disabled);
  const disabledClicks = clickResults.filter((result) => result.disabled);
  if (viewClicks.length < 8) issues.push(issue("view-click-coverage", "medium", "Insufficient view-click coverage", `Expected at least 8 enabled view action clicks, observed ${viewClicks.length}.`));
  if (disabledClicks.length < 1) issues.push(issue("disabled-click-coverage", "medium", "No disabled action click covered", "Smoke should cover at least one disabled coach action."));

  const checklist = {
    target_metadata_present: captures.every((capture) => capture.actions.every((action) => action.label && action.disabledData && ("view" in action) && ("event" in action))),
    enabled_view_actions_navigate: viewClicks.length >= 8 && viewClicks.every((result) => result.passed),
    disabled_actions_are_inert: disabledClicks.length >= 1 && disabledClicks.every((result) => result.passed),
    version_visible: captures.every((capture) => capture.coachId),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.5 workflow coach action execution smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    captures,
    click_results: clickResults,
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
