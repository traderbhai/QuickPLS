import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const ARTIFACTS = path.join(RESULTS, "screens", "v221", "command-handoff");
const OUTPUT = path.join(RESULTS, "v221_command_handoff_smoke.json");
const PORT = 53221;
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

async function currentCoach(page) {
  return page.evaluate(() => {
    const coach = document.querySelector(".workspace-coach");
    const buttons = coach ? Array.from(coach.querySelectorAll("button")).map((button) => ({
      text: button.textContent?.replace(/\s+/g, " ").trim() ?? "",
      disabled: button.hasAttribute("disabled"),
      title: button.getAttribute("title") ?? "",
    })) : [];
    return {
      id: coach?.getAttribute("data-coach-id") ?? "",
      aria: coach?.getAttribute("aria-label") ?? "",
      buttons,
      body: document.body.innerText,
    };
  });
}

async function clickCoachButton(page, label) {
  const buttons = page.locator(".workspace-coach button");
  const count = await buttons.count();
  for (let index = 0; index < count; index += 1) {
    const button = buttons.nth(index);
    const text = (await button.innerText()).replace(/\s+/g, " ").trim();
    if (text.includes(label)) {
      await button.click();
      await page.waitForTimeout(300);
      return true;
    }
  }
  return false;
}

function issue(id, severity, title, detail) {
  return { id, severity, title, detail };
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
  await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__), null, { timeout: 10_000 });

  await page.evaluate(() => {
    window.__quickplsCommandSmoke = [];
    [
      "quickpls:run-analysis",
      "quickpls:save-project",
      "quickpls:open-project",
      "quickpls:open-demo-project",
      "quickpls:import-data",
    ].forEach((event) => window.addEventListener(event, () => window.__quickplsCommandSmoke.push(event)));
  });

  const commandEvents = await page.evaluate(() => {
    [
      "quickpls:save-project",
      "quickpls:open-project",
      "quickpls:open-demo-project",
      "quickpls:import-data",
    ].forEach((event) => window.dispatchEvent(new CustomEvent(event)));
    return window.__quickplsCommandSmoke;
  });

  await setView(page, "data");
  const dataBefore = await currentCoach(page);
  const dataClicked = await clickCoachButton(page, "Open Model");
  const dataAfter = await page.evaluate(() => document.body.innerText);
  await page.screenshot({ path: path.join(ARTIFACTS, "1440x900_data_to_model.png"), fullPage: true });
  captures.push({ name: "data_to_model", before: dataBefore.id, clicked: dataClicked, afterContainsModel: /SEM EXPLORER|Constructs|Model-only diagram/i.test(dataAfter) });

  await setView(page, "analyses");
  const setupBefore = await currentCoach(page);
  const setupClicked = setupBefore.id === "setup-ready-run"
    ? await clickCoachButton(page, "Run now")
    : await clickCoachButton(page, "Open Data");
  const setupAfter = await page.evaluate(() => document.body.innerText);
  await page.screenshot({ path: path.join(ARTIFACTS, "1440x900_setup_run_command.png"), fullPage: true });
  captures.push({
    name: "setup_run_command",
    before: setupBefore.id,
    clicked: setupClicked,
    blockedStateExplained: setupBefore.id === "setup-needs-work" && setupBefore.buttons.some((button) => button.text.length > 0),
    afterContainsRun: /Run analysis|Calculation package|Run selected method/i.test(setupAfter),
  });

  await setView(page, "settings");
  const settingsBefore = await currentCoach(page);
  const trustClicked = await clickCoachButton(page, "Open Trust Center");
  const trustAfter = await page.evaluate(() => document.body.innerText);
  await page.screenshot({ path: path.join(ARTIFACTS, "1440x900_settings_to_trust.png"), fullPage: true });
  captures.push({ name: "settings_to_trust", before: settingsBefore.id, clicked: trustClicked, afterContainsTrust: /Trust Center|Validation artifacts|known limitations/i.test(trustAfter) });

  const bodyText = await page.evaluate(() => document.body.innerText);
  if (!bodyText.includes("v2.2.1 command handoff")) issues.push(issue("version-label", "high", "Version label mismatch", "The rendered top bar should show v2.2.1 command handoff."));
  if (dataBefore.aria !== "Workflow coach" || setupBefore.aria !== "Workflow coach" || settingsBefore.aria !== "Workflow coach") issues.push(issue("coach-aria", "medium", "Workflow coach aria marker missing", "The command handoff smoke relies on a stable workflow coach accessible name."));
  if (!commandEvents.includes("quickpls:save-project") || !commandEvents.includes("quickpls:open-project") || !commandEvents.includes("quickpls:open-demo-project") || !commandEvents.includes("quickpls:import-data")) issues.push(issue("command-events", "high", "Shared command events did not dispatch", `Observed events: ${commandEvents.join(", ")}`));
  if (!captures.every((item) => item.clicked || item.blockedStateExplained)) issues.push(issue("coach-clicks", "high", "Coach actions were not clickable", "Every tested coach action should be reachable by its rendered button label or explain its blocked state."));
  if (!captures.find((item) => item.name === "data_to_model")?.afterContainsModel) issues.push(issue("data-handoff", "high", "Data coach did not open Model", "The Data coach primary action should hand off to the Model workspace."));
  if (!captures.find((item) => item.name === "setup_run_command")?.afterContainsRun) issues.push(issue("setup-handoff", "high", "Setup coach did not hand off to Run", "The Setup coach primary action should keep the researcher on the calculation path."));
  if (!captures.find((item) => item.name === "settings_to_trust")?.afterContainsTrust) issues.push(issue("settings-handoff", "medium", "Settings coach did not open Trust Center", "The Settings coach secondary action should open Trust Center."));
  if (/RÃƒ|RÃ‚|Ã‚Â²/.test(bodyText)) issues.push(issue("mojibake", "high", "Mojibake visible", "Rendered text includes corrupted encoding artifacts."));
  if (/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(bodyText)) issues.push(issue("smartpls-claim", "high", "Unsupported SmartPLS equivalence claim", "Rendered text implies SmartPLS equivalence."));

  const checklist = {
    shared_events_observed: commandEvents.length >= 4,
    coach_navigation_handoffs_work: captures.every((item) => item.clicked || item.blockedStateExplained),
    no_console_errors: errors.length === 0,
    no_issues: issues.length === 0,
  };
  const result = {
    schema_version: 1,
    target: "QuickPLS v2.2.1 command handoff smoke",
    passed: Object.values(checklist).every(Boolean),
    generated_at: new Date().toISOString(),
    checklist,
    command_events: commandEvents,
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
