#!/usr/bin/env node
/**
 * QuickPLS 2.55 live candidate lifecycle proof.
 *
 * The underlying moderation journey creates a real model, starts an actual
 * current-engine calculation, waits for categorized Results, saves through the
 * native dialog, and reopens from a fresh isolated application process. It is
 * intentionally separate from the archive breadth index: frozen evidence may
 * prove unchanged presentation breadth, never replace this live lifecycle.
 */

import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RESULTS = path.join(ROOT, "validation", "results");
const UNDERLYING_DRIVER = path.join(ROOT, "validation", "v253_mediation_moderation_packaged_smoke.mjs");

function assert(condition, message) { if (!condition) throw new Error(message); }
function inside(parent, candidate) {
  const relative = path.relative(path.resolve(parent), path.resolve(candidate));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}
function parseArgs(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) throw new Error(`Unexpected positional argument: ${token}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${token}`);
    values[token.slice(2)] = value;
    index += 1;
  }
  for (const key of ["phase", "endpoint", "evidence-dir", "project-path", "python"]) {
    if (!values[key]) throw new Error(`--${key} is required.`);
  }
  if (!new Set(["execute", "reopen"]).has(values.phase)) throw new Error("--phase execute or --phase reopen is required.");
  return values;
}
function runNode(argumentsList) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, argumentsList, { cwd: ROOT, windowsHide: true, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code, signal) => resolve({ code, signal }));
  });
}

async function namedObservation({ caseId, operation, observed, screenshot }) {
  const screenshotPath = path.resolve(ROOT, screenshot);
  assert(inside(RESULTS, screenshotPath), `${caseId} screenshot escapes validation/results.`);
  const screenshotBytes = await fs.readFile(screenshotPath);
  return {
    schema_version: 1,
    case_id: caseId,
    operation,
    assertion: {
      id: `${operation}:${caseId}`,
      passed: true,
      expected: observed,
      observed,
    },
    screenshot: {
      path: screenshotPath,
      sha256: crypto.createHash("sha256").update(screenshotBytes).digest("hex"),
    },
  };
}

function screenshotById(underlying, id) {
  return (underlying?.screenshots ?? []).find((value) => String(value).replace(/\\/g, "/").endsWith(`/screens/${id}.png`));
}

const args = parseArgs(process.argv.slice(2));
const evidenceDir = path.resolve(args["evidence-dir"]);
const projectPath = path.resolve(args["project-path"]);
assert(inside(RESULTS, evidenceDir), "--evidence-dir must remain below validation/results.");
assert(inside(RESULTS, projectPath), "--project-path must remain below validation/results.");
const underlyingReportPath = path.join(evidenceDir, "v253_mediation_moderation_packaged_smoke.json");
const reportPath = path.join(evidenceDir, "v255_live_calculation_lifecycle_smoke.json");
const report = await fs.stat(reportPath).then(async () => JSON.parse(await fs.readFile(reportPath, "utf8")), () => ({
  schema_version: 1,
  suite_id: "quickpls_v255_live_calculation_lifecycle_smoke_v1",
  target_release: "2.55.0",
  version_authority: "2.54.0 until the 2.55 release gate succeeds",
  current_candidate_journey: true,
  source_driver: "validation/v253_mediation_moderation_packaged_smoke.mjs",
  complete: false,
  passed: false,
  phases: {},
  named_evidence_observations: [],
  failures: [],
}));

const childArgs = [UNDERLYING_DRIVER, "--phase", args.phase, "--endpoint", args.endpoint, "--evidence-dir", evidenceDir, "--project-path", projectPath, "--python", path.resolve(args.python)];
const outcome = await runNode(childArgs);
const phase = { phase: args.phase, child_exit_code: outcome.code, child_signal: outcome.signal, passed: false };
let underlying;
try {
  assert(outcome.code === 0 && !outcome.signal, `Underlying current-candidate journey failed: ${JSON.stringify(outcome)}`);
  underlying = JSON.parse(await fs.readFile(underlyingReportPath, "utf8"));
  const underlyingPhase = underlying.phases?.[args.phase];
  assert(underlyingPhase?.passed === true, `Underlying report did not pass ${args.phase}.`);
  if (args.phase === "execute") {
    assert(underlying.phases.execute?.checks?.terminalState === "completed", "Live calculation did not reach completed Results.");
    assert(underlying.phases.execute?.checks?.save?.target === projectPath, "Live calculation did not save to the isolated project path.");
  } else {
    assert(underlying.phases.execute?.passed === true, "Fresh reopen was attempted without a passing live execute phase.");
    assert(underlying.phases.reopen?.checks?.sameCanonicalIdentity === true, "Fresh reopen changed canonical result identity.");
  }
  phase.passed = true;
  phase.underlying_report = path.relative(ROOT, underlyingReportPath).split(path.sep).join("/");
  phase.underlying_phase = underlyingPhase.checks;
} catch (error) {
  phase.failure = error instanceof Error ? error.message : String(error);
}
report.phases[args.phase] = phase;
report.complete = report.phases.execute?.passed === true && report.phases.reopen?.passed === true;
report.passed = report.complete;
report.failures = Object.entries(report.phases).flatMap(([name, value]) => value.passed ? [] : [`${name}: ${value.failure ?? "phase did not pass"}`]);
report.named_evidence_observations = [];
if (report.complete && underlying?.passed === true) {
  const progressScreenshot = screenshotById(underlying, "04-progress");
  const resultsScreenshot = screenshotById(underlying, "05-results");
  const reopenScreenshot = screenshotById(underlying, "06-reopen");
  assert(progressScreenshot && resultsScreenshot && reopenScreenshot, "Live lifecycle evidence lacks progress, Results, or reopen screenshots.");
  report.named_evidence_observations.push(await namedObservation({
    caseId: "cross_method:observability:running or progress screenshot",
    operation: "capture_observability_state",
    observed: {
      progress_text: underlying.phases.execute.checks.progress,
      terminal_state: underlying.phases.execute.checks.terminalState,
    },
    screenshot: progressScreenshot,
  }));
  report.named_evidence_observations.push(await namedObservation({
    caseId: "cross_method:persistence:save and fresh reopen",
    operation: "exercise_persistence",
    observed: {
      saved_target: underlying.phases.execute.checks.save.target,
      same_canonical_identity: underlying.phases.reopen.checks.sameCanonicalIdentity,
      reopened_identity: underlying.phases.reopen.checks.results.identity,
    },
    screenshot: reopenScreenshot,
  }));
  report.named_evidence_observations.push(await namedObservation({
    caseId: "cross_method:packaged:offline request observation",
    operation: "verify_packaged_candidate",
    observed: {
      execute: underlying.phases.execute.offline,
      reopen: underlying.phases.reopen.offline,
    },
    screenshot: resultsScreenshot,
  }));
}
report.completed_at = new Date().toISOString();
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
if (!phase.passed) process.exit(1);
console.log(JSON.stringify({ passed: true, phase: args.phase, complete: report.complete, report: reportPath }, null, 2));
