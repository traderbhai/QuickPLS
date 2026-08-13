import path from "node:path";
import { ensureDir, issuesFromChecklist, RESULTS, withPreviewPage, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const screenDir = path.join(RESULTS, "screens", "v2270", "run-monitor");
const resultPath = path.join(RESULTS, "v2270_run_monitor_smoke.json");

async function captureState(page, state, viewportLabel) {
  await page.evaluate((nextState) => {
    window.__QUICKPLS_SMOKE__?.loadDiagramFixture?.("mediation");
    window.__QUICKPLS_SMOKE__?.setView?.("run");
    window.__QUICKPLS_SMOKE__?.setRunMonitorFixture?.(nextState);
  }, state);
  await page.waitForSelector('[data-v227-run-monitor="true"]', { timeout: 10_000 });
  await page.waitForTimeout(120);
  const screenshot = path.join(screenDir, `${viewportLabel}-${state}.png`);
  await page.screenshot({ path: screenshot, fullPage: false });
  const snapshot = await page.evaluate(() => {
    const host = document.querySelector('[data-v227-run-monitor="true"]');
    const text = document.body.innerText;
    const html = host?.outerHTML ?? "";
    return {
      hasRunMonitor: Boolean(host),
      steps: Array.from(document.querySelectorAll(".run-v227-step")).map((node) => node.textContent?.replace(/\s+/g, " ").trim()).filter(Boolean),
      progressPanel: document.querySelector(".run-v227-progress-panel")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      settingsPanel: document.querySelector(".run-v227-settings-panel")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      footer: document.querySelector(".run-v227-footer")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      hasHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth + 4,
      hasMojibake: text.includes("RÂ²") || text.includes("RÃ") || html.includes("RÂ²") || html.includes("RÃ"),
      hasProgressTrack: Boolean(document.querySelector(".run-v227-progress-track span")),
      hasLog: Boolean(document.querySelector(".run-v227-log article")),
    };
  });
  return { state, viewport: viewportLabel, screenshot, snapshot };
}

const captures = [];

await ensureDir(screenDir);

for (const viewport of [
  { label: "1440x900", width: 1440, height: 900 },
  { label: "1280x800", width: 1280, height: 800 },
]) {
  await withPreviewPage({
    port: viewport.width === 1440 ? 5270 : 5271,
    viewport: { width: viewport.width, height: viewport.height },
    run: async ({ page, errors }) => {
      captures.push(await captureState(page, "blocked", viewport.label));
      captures.push(await captureState(page, "running", viewport.label));
      captures.push(await captureState(page, "failed", viewport.label));
      captures.push(await captureState(page, "completed", viewport.label));
      if (errors.length) {
        captures.push({ state: "console", viewport: viewport.label, screenshot: null, snapshot: { errors } });
      }
    },
  });
}

const allSnapshots = captures.filter((item) => item.snapshot?.hasRunMonitor);
const checklist = {
  rendered_run_monitor_marker: allSnapshots.length >= 8,
  procedure_has_five_steps: allSnapshots.every((item) => item.snapshot.steps.length === 5),
  progress_panel_has_track_and_log: allSnapshots.every((item) => item.snapshot.hasProgressTrack && item.snapshot.hasLog),
  settings_summary_has_key_fields: allSnapshots.every((item) => ["Method", "Scope", "Seed", "Workers", "Data fingerprint", "Recipe fingerprint", "Outputs produced"].every((label) => item.snapshot.settingsPanel.includes(label))),
  footer_has_run_handoff_actions: allSnapshots.every((item) => item.snapshot.footer.includes("Setup") && item.snapshot.footer.includes("Open results") && item.snapshot.footer.includes("Prepare report")),
  blocked_running_failed_completed_states_visible: ["blocked", "running", "failed", "completed"].every((state) => captures.some((item) => item.state === state && item.snapshot?.progressPanel?.toLowerCase().includes(state === "running" ? "algorithm" : state))),
  no_rendered_mojibake: allSnapshots.every((item) => !item.snapshot.hasMojibake),
  no_document_horizontal_overflow: allSnapshots.every((item) => !item.snapshot.hasHorizontalOverflow),
  no_console_errors: !captures.some((item) => item.state === "console"),
};

const passed = Object.values(checklist).every(Boolean);
const issues = issuesFromChecklist(checklist);

await writeJson(resultPath, {
  id: "v2270_run_monitor_smoke",
  milestone: "v2_27_0_calculation_run_monitor",
  passed,
  generated_at: new Date().toISOString(),
  checklist,
  issues,
  captures,
});

if (!passed) {
  console.error(JSON.stringify({ passed, issues, checklist }, null, 2));
  process.exit(1);
}

console.log(`v2.27 run monitor smoke passed: ${resultPath}`);
