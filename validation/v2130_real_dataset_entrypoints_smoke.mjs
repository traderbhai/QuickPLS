import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");

const trust = read("src/components/TrustCenterWorkspace.tsx");
const settings = read("src/components/SettingsWorkspace.tsx");
const home = read("src/components/OnboardingWorkspace.tsx");
const active = read("docs/V2_ACTIVE_MILESTONE.md");

const checks = [
  ["trust center entrypoint", trust.includes("Real dataset review protocol") && trust.includes('data-real-dataset-protocol-entrypoint="trust-center"')],
  ["trust center links protocol artifacts", trust.includes("docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md") && trust.includes("validation/templates/real_dataset_issue_register_template.json")],
  ["settings entrypoint", settings.includes("Real dataset review") && settings.includes('data-real-dataset-protocol-entrypoint="settings"')],
  ["settings privacy rules visible", ["Never commit", "Redact first", "Anonymized", "Fixtures only"].every((token) => settings.includes(token))],
  ["home private dataset notice", home.includes("Reviewing a private dataset?") && home.includes("Open protocol") && home.includes('start("trust")')],
  ["active tracker current", active.includes("v2_13_0_real_dataset_protocol_entrypoints")],
  ["no SmartPLS equivalence claim", ![trust, settings, home].some((source) => /identical to smartpls|smartpls equivalent|equivalent to smartpls/i.test(source))],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_13_0_real_dataset_protocol_entrypoints",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v2130_real_dataset_entrypoints_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.13 real dataset protocol entrypoints smoke passed");
