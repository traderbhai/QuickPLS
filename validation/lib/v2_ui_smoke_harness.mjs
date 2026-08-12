import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
export const RESULTS = path.join(ROOT, "validation", "results");

export async function ensureDir(dir) {
  await fs.mkdir(dir, { recursive: true });
}

export async function writeJson(file, payload) {
  await ensureDir(path.dirname(file));
  await fs.writeFile(file, JSON.stringify(payload, null, 2));
}

export function startPreview(port) {
  const server = spawn("cmd.exe", ["/c", `npx vite preview --host 127.0.0.1 --port ${port} --strictPort`], {
    cwd: ROOT,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let logs = "";
  server.stdout.on("data", (data) => { logs += data.toString(); });
  server.stderr.on("data", (data) => { logs += data.toString(); });
  return { server, logs: () => logs };
}

export async function waitForPreview(url, logs) {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1500) });
      if (response.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Timed out waiting for Vite preview. ${logs().slice(-1200)}`);
}

export function stopPreview(server, port) {
  if (server.pid) {
    try {
      execFileSync("taskkill.exe", ["/PID", String(server.pid), "/T", "/F"], { stdio: "ignore" });
    } catch {
      server.kill();
    }
    try {
      execFileSync("powershell.exe", ["-NoProfile", "-Command", `(Get-NetTCPConnection -LocalPort ${port} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess -Unique) | ForEach-Object { Stop-Process -Id $_ -Force }`], { stdio: "ignore" });
    } catch {
      // Best-effort cleanup for detached Vite children.
    }
  } else {
    server.kill();
  }
}

export async function withPreviewPage({ port, viewport = { width: 1440, height: 900 }, run }) {
  const url = `http://127.0.0.1:${port}/`;
  const { server, logs } = startPreview(port);
  let browser;
  try {
    await waitForPreview(url, logs);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport, deviceScaleFactor: 1 });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.goto(`${url}?quickpls_smoke=1`, { waitUntil: "domcontentloaded", timeout: 45_000 });
    await page.waitForFunction(() => Boolean(window.__QUICKPLS_SMOKE__?.loadEmptyProject), null, { timeout: 10_000 });
    await page.evaluate(() => window.__QUICKPLS_SMOKE__?.loadEmptyProject?.());
    await page.waitForTimeout(250);
    return await run({ page, errors, url });
  } finally {
    if (browser) await browser.close();
    stopPreview(server, port);
  }
}

export async function collectV2ShellSnapshot(page) {
  return page.evaluate(() => {
    const text = document.body.innerText;
    const run = document.querySelector(".run-button");
    const cluster = document.querySelector(".command-run-cluster");
    const chip = document.querySelector(".command-blocker-chip");
    return {
      view: window.__QUICKPLS_SMOKE__?.getView?.() ?? "",
      destinationContext: window.__QUICKPLS_SMOKE__?.getWorkflowDestinationContext?.() ?? null,
      version: document.querySelector(".alpha-mark")?.textContent?.trim() ?? "",
      title: document.querySelector(".title-bar strong")?.textContent?.trim() ?? "",
      project: document.querySelector(".project-title")?.textContent?.trim() ?? "",
      rail: Array.from(document.querySelectorAll(".nav-rail button")).map((node) => node.textContent?.trim()).filter(Boolean),
      workflow: Array.from(document.querySelectorAll(".workflow-step")).map((node) => node.textContent?.replace(/\s+/g, " ").trim()).filter(Boolean),
      commandState: cluster?.getAttribute("data-command-bar-state") ?? "",
      commandMethod: cluster?.getAttribute("data-run-method") ?? "",
      runState: run?.getAttribute("data-run-state") ?? "",
      runDisabled: run instanceof HTMLButtonElement ? run.disabled : null,
      runReason: run?.getAttribute("data-run-disabled-reason") ?? "",
      blockerId: chip?.getAttribute("data-run-blocker-id") ?? "",
      blockerView: chip?.getAttribute("data-run-blocker-view") ?? "",
      blockerAction: chip?.getAttribute("data-run-blocker-action") ?? "",
      blockerText: chip?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      hasTrust: text.includes("Trust"),
      hasSettings: text.includes("Settings"),
      hasR2Mojibake: text.includes("RÂ²") || text.includes("RÃ") || text.includes("Ã‚"),
      hasSmartPlsEquivalence: /identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(text),
      body: text,
    };
  });
}

export function evaluateV2ShellIntegrity(snapshot, expectedVersion) {
  return {
    version_label_current: snapshot.version === expectedVersion,
    shell_identity_present: snapshot.title === "QuickPLS" && snapshot.project.endsWith(".qpls"),
    primary_workflow_present: ["Data", "Model", "Setup", "Run", "Results", "Report"].every((label) => snapshot.workflow.some((item) => item.includes(label))),
    secondary_rail_routes_present: snapshot.hasTrust && snapshot.hasSettings,
    command_bar_readiness_metadata_present: snapshot.commandState === "blocked" && snapshot.runState === "blocked" && snapshot.runReason.length > 0 && snapshot.blockerId === "runtime" && snapshot.blockerView === "models",
    blocker_text_specific: snapshot.blockerText.includes("Run disabled") && snapshot.blockerText.includes("Runtime"),
    no_rendered_mojibake: !snapshot.hasR2Mojibake,
    no_smartpls_equivalence_claim: !snapshot.hasSmartPlsEquivalence,
  };
}

export function issuesFromChecklist(checklist) {
  return Object.entries(checklist)
    .filter(([, passed]) => !passed)
    .map(([id]) => ({ id, severity: "high", detail: `Failed check: ${id}` }));
}
