import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultsDir = path.join(root, "validation", "results");
fs.mkdirSync(resultsDir, { recursive: true });

const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const readJson = (relative) => JSON.parse(read(relative));

const template = readJson("validation/templates/real_dataset_feedback_triage_template.json");
const active = read("docs/V2_ACTIVE_MILESTONE.md");
const protocol = read("docs/V2_14_0_REAL_DATASET_FEEDBACK_TRIAGE.md");
const registry = readJson("validation/development_slices.json");

const backlog = {
  schema_version: 1,
  milestone: "v2_14_0_real_dataset_feedback_triage",
  generated_at: new Date().toISOString(),
  source_evidence: [
    "docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md",
    "docs/V2_13_0_REAL_DATASET_PROTOCOL_ENTRYPOINTS.md",
    "validation/templates/real_dataset_feedback_triage_template.json",
  ],
  privacy_boundary: {
    raw_private_data_in_repo: false,
    value_revealing_screenshots_in_repo: false,
    private_projects_in_repo: false,
    automated_gates_use_private_data: false,
  },
  triage_lanes: [
    {
      id: "launch_blockers",
      priority: "P0",
      next_action: "Only start a fix milestone when a synthetic reproduction or redacted manual evidence exists.",
    },
    {
      id: "workflow_and_method_guidance",
      priority: "P1",
      next_action: "Batch Setup, Data, Model, Results, and Report confusion into grouped UI milestones.",
    },
    {
      id: "visual_polish_and_export_friction",
      priority: "P2",
      next_action: "Batch visual and export refinements by workspace, not as micro-fixes.",
    },
    {
      id: "statistical_evidence_gap",
      priority: "separate",
      next_action: "Keep out of frontend-only milestones; route to validation/method-promotion work.",
    },
  ],
  next_grouped_milestone_policy: "Choose from anonymized triage notes, generated fixtures, or explicit user feedback; do not start a micro-fix.",
};

fs.writeFileSync(
  path.join(resultsDir, "v2140_real_dataset_triage_backlog.json"),
  JSON.stringify(backlog, null, 2),
);

const checks = [
  ["template exists and has privacy rules", template.privacy_rules?.only_anonymized_notes === true],
  ["template blocks raw data", template.privacy_rules?.raw_data_committed === false],
  ["template has finding categories", template.findings?.[0]?.category === "workflow_friction"],
  ["protocol names intake workflow", protocol.includes("Intake Workflow") && protocol.includes("synthetic fixtures")],
  ["active tracker names v2.14", active.includes("v2_14_0_real_dataset_feedback_triage")],
  ["registry current stage v2.14", registry.current_stage === "v2_14_0_real_dataset_feedback_triage"],
  ["backlog has no private persistence", Object.values(backlog.privacy_boundary).every((value) => value === false)],
  ["no SmartPLS equivalence claim", !/identical to smartpls|smartpls equivalent|equivalent to smartpls/i.test(protocol + active)],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_14_0_real_dataset_feedback_triage",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(
  path.join(resultsDir, "v2140_real_dataset_triage_smoke.json"),
  JSON.stringify(report, null, 2),
);

if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}

console.log("v2.14 real dataset feedback triage smoke passed");
