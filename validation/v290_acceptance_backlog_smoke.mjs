import fs from "node:fs/promises";
import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const TARGET = "v2_9_0_acceptance_backlog_and_next_pass";
const ARTIFACTS = path.join(RESULTS, "screens", "v290", "acceptance-backlog");
const OUTPUT = path.join(RESULTS, "v290_acceptance_backlog_smoke.json");
const BACKLOG = path.join(RESULTS, "v290_acceptance_backlog.json");

await ensureDir(ARTIFACTS);

const views = [
  { view: "welcome", label: "Home", selector: "[data-nav-view='welcome']", family: "support" },
  { view: "data", label: "Data", selector: "[data-nav-view='data']", family: "workflow" },
  { view: "models", label: "Model", selector: "[data-nav-view='models']", family: "workflow" },
  { view: "analyses", label: "Setup", selector: "[data-nav-view='analyses']", family: "workflow" },
  { view: "run", label: "Run", selector: "[data-nav-view='run']", family: "workflow" },
  { view: "runs", label: "Results", selector: "[data-nav-view='runs']", family: "workflow" },
  { view: "reports", label: "Report", selector: "[data-nav-view='reports']", family: "workflow" },
  { view: "trust", label: "Trust Center", selector: "[data-nav-view='trust']", family: "support" },
  { view: "settings", label: "Settings", selector: "[data-nav-view='settings']", family: "support" },
];

function backlogFromEvidence({ states, priorRegister }) {
  const anyOverflow = states.some((item) => Math.max(item.state.documentOverflowX, item.state.pageHostOverflowX) > 2);
  const priorIssues = Array.isArray(priorRegister?.open_issues) ? priorRegister.open_issues : [];
  return {
    schema_version: 1,
    milestone: TARGET,
    generated_at: new Date().toISOString(),
    source_evidence: [
      "validation/results/v270_visual_issue_register.json",
      "validation/results/v290_acceptance_backlog_smoke.json",
      "validation/results/screens/v290/acceptance-backlog/",
      "docs/V2_ACTIVE_MILESTONE.md",
    ],
    current_shell_health: priorIssues.length || anyOverflow ? "review" : "clear_for_next_grouped_pass",
    workstreams: [
      {
        id: "results_report_research_table_pass",
        decision: "do_next",
        priority: 1,
        target: "Results and Report",
        problem: "The next highest-value researcher experience pass is table scanning, interpretation density, and report/export confidence across real-like saved runs.",
        acceptance: [
          "Results tables use the shared research table shell where practical.",
          "Report export preview and selected-run context remain visible before export.",
          "Interpretation is value-specific, deduplicated, and optional in numeric exports.",
        ],
        evidence_needed: [
          "rendered 1440x900 and 1280x800 screenshots",
          "targeted Results/Report smoke JSON",
          "static audit proving no backend/numerical files were touched",
        ],
      },
      {
        id: "method_setup_applicability_followup",
        decision: "do_next",
        priority: 2,
        target: "Setup, Data, Model",
        problem: "Method availability should remain understandable when users bring private datasets, covariance/correlation input, group columns, binary outcomes, or unsupported model shapes.",
        acceptance: [
          "Recommended, available, needs setup, not applicable, experimental, and unsupported states are visible with exact reasons.",
          "Top-bar method choices stay conservative outside Setup.",
          "Data and Model panels explain what can be done next with the current project.",
        ],
        evidence_needed: [
          "method applicability fixture matrix",
          "Setup rendered smoke for standard, group, regression, NCA, and unsupported CB-SEM shapes",
          "no overclaiming of validated scope",
        ],
      },
      {
        id: "real_dataset_review_protocol",
        decision: "do_next",
        priority: 3,
        target: "Data, Setup, Results, Report",
        problem: "Before additional visual polish, QuickPLS needs a repeatable protocol for reviewing real researcher datasets without checking private data into the repository.",
        acceptance: [
          "Private/manual dataset review checklist exists.",
          "Synthetic and bundled fixtures remain the automated gate inputs.",
          "Manual notes separate product issues from statistical evidence gaps.",
        ],
        evidence_needed: [
          "docs/manual dataset review checklist",
          "generated anonymized issue-register template",
          "clear no-private-data persistence rule",
        ],
      },
      {
        id: "sem_designer_shell_only_polish",
        decision: "defer",
        priority: 4,
        target: "Model shell around SEM Designer",
        problem: "The SEM Designer core is currently considered acceptable by the user; only surrounding shell, focus mode, and handoff context should be changed unless explicitly requested.",
        acceptance: [
          "No change to SEM core geometry or interaction without explicit user request.",
          "Any shell polish preserves existing drag/drop, path routing, overlays, and SVG parity.",
        ],
        evidence_needed: [
          "Model shell smoke",
          "SEM Designer regression screenshot",
        ],
      },
      {
        id: "avoid_microfix_drift",
        decision: "do_not_do",
        priority: 99,
        target: "Development process",
        problem: "Small ungrouped visual tweaks consume time and tokens without producing a release-quality checkpoint.",
        acceptance: [
          "No new milestone starts without target screens, smoke/audit, and final gate.",
          "No artifact build before milestone gate and pre-artifact checks are clear.",
        ],
        evidence_needed: [
          "docs/V2_ACTIVE_MILESTONE.md operating rules",
          "versioned artifact manifest only after completed milestones",
        ],
      },
    ],
  };
}

async function collectViewState(page, target, screenshotDir) {
  await page.locator(target.selector).click();
  await page.waitForTimeout(200);
  const shell = await collectV2ShellSnapshot(page);
  const state = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    const pageHost = document.querySelector(".page-host");
    const header = document.querySelector(".workspace-page h1, .home-header h1, .qpls2-page-shell h1");
    const tables = [...document.querySelectorAll("table")];
    const panels = [...document.querySelectorAll(".q2-panel, .q2-card, .results-card, .home-action-card")];
    return {
      header: header?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      documentOverflowX: root.scrollWidth - root.clientWidth,
      pageHostOverflowX: pageHost ? pageHost.scrollWidth - pageHost.clientWidth : 0,
      panelCount: panels.length,
      tableCount: tables.length,
      hasVisiblePrimaryAction: [...document.querySelectorAll("button")].some((button) => !button.disabled && button.textContent?.trim()),
      bodyLength: document.body.innerText.length,
    };
  });
  const screenshot = path.join(screenshotDir, `${target.family}-${target.view}.png`);
  await page.screenshot({ path: screenshot, fullPage: true });
  return { ...target, screenshot, state: { ...shell, ...state } };
}

async function captureViewport({ viewport, port, label }) {
  const dir = path.join(ARTIFACTS, label);
  await ensureDir(dir);
  return withPreviewPage({
    port,
    viewport,
    run: async ({ page, errors }) => {
      const states = [];
      for (const target of views) {
        states.push(await collectViewState(page, target, dir));
      }
      const checklist = {
        all_target_views_captured: states.length === views.length,
        all_primary_views_have_headers_or_body: states.every((item) => item.state.header || item.state.bodyLength > 100),
        no_horizontal_document_overflow: states.every((item) => Math.max(item.state.documentOverflowX, item.state.pageHostOverflowX) <= 2),
        no_rendered_mojibake: states.every((item) => !item.state.hasR2Mojibake),
        no_smartpls_equivalence_claim: states.every((item) => !item.state.hasSmartPlsEquivalence),
        no_console_errors: errors.length === 0,
      };
      return {
        viewport,
        screenshot_dir: dir,
        states,
        checklist,
        issues: issuesFromChecklist(checklist),
        errors,
      };
    },
  });
}

const viewports = [
  { label: "1440x900", viewport: { width: 1440, height: 900 }, port: 53290 },
  { label: "1280x800", viewport: { width: 1280, height: 800 }, port: 53291 },
];

const priorRegisterPath = path.join(RESULTS, "v270_visual_issue_register.json");
let priorRegister = {};
try {
  priorRegister = JSON.parse(await fs.readFile(priorRegisterPath, "utf8"));
} catch {
  priorRegister = {};
}

const runs = [];
for (const config of viewports) {
  runs.push(await captureViewport(config));
}

const issues = runs.flatMap((run) => [
  ...run.issues.map((issue) => ({ ...issue, viewport: `${run.viewport.width}x${run.viewport.height}` })),
  ...run.errors.map((error) => ({
    id: "console_error",
    severity: "high",
    viewport: `${run.viewport.width}x${run.viewport.height}`,
    detail: error,
  })),
]);

const states = runs.flatMap((run) => run.states);
const backlog = backlogFromEvidence({ states, priorRegister });
const backlogChecks = {
  has_do_next_workstreams: backlog.workstreams.filter((item) => item.decision === "do_next").length >= 3,
  has_defer_workstream: backlog.workstreams.some((item) => item.decision === "defer"),
  has_do_not_do_workstream: backlog.workstreams.some((item) => item.decision === "do_not_do"),
  every_workstream_has_acceptance: backlog.workstreams.every((item) => item.acceptance.length >= 2 && item.evidence_needed.length >= 2),
};

const result = {
  schema_version: 1,
  target: "QuickPLS v2.9.0 acceptance backlog smoke",
  passed: issues.length === 0 && Object.values(backlogChecks).every(Boolean),
  generated_at: new Date().toISOString(),
  runs,
  issues,
  backlog_checks: backlogChecks,
  screenshots_dir: ARTIFACTS,
  backlog: BACKLOG,
};

await writeJson(OUTPUT, result);
await fs.writeFile(BACKLOG, JSON.stringify(backlog, null, 2));
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
