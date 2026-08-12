import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (...parts) => fs.readFileSync(path.join(root, ...parts), "utf8");

const app = read("src", "v2", "NativePrototypeApp.tsx");
const pkg = JSON.parse(read("package.json"));
const registry = JSON.parse(read("validation", "development_slices.json"));
const roadmap = read("crates", "qpls-core", "src", "roadmap.rs");
const milestoneDoc = read("docs", "V2_45_0_MOCKUP_VISIBLE_FEATURE_COMPLETION.md");
const backlog = read("docs", "V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md");
const tauriConf = JSON.parse(read("src-tauri", "tauri.conf.json"));
const cargoToml = read("Cargo.toml");

const scripts = pkg.scripts ?? {};
const checks = [
  ["package version", pkg.version === "2.45.0"],
  ["tauri version", tauriConf.version === "2.45.0"],
  ["cargo version", /version = "2\.45\.0"/.test(cargoToml)],
  ["artifact label", /v2_45_0_mockup_visible_feature_completion/.test(scripts["qpls:release:artifacts"] ?? "")],
  ["smoke script", scripts["qpls:v245:mockup-feature-smoke"] === "node validation/v245_mockup_feature_completion_smoke.mjs"],
  ["audit script", scripts["qpls:v245:mockup-feature-audit"] === "node validation/v245_mockup_feature_completion_audit.mjs"],
  ["aggregate script", /qpls:v245:mockup-feature-smoke/.test(scripts["qpls:v245:mockup-feature-completion"] ?? "")],
  ["registry current stage", registry.current_stage === "v2_45_0_mockup_visible_feature_completion"],
  ["registry slice", registry.slices.some((slice) => slice.id === "v2_45_0_mockup_visible_feature_completion" && slice.status === "validated")],
  ["roadmap current stage", /v2_45_0_mockup_visible_feature_completion/.test(roadmap)],
  ["milestone doc", /Data Workbench tabs/.test(milestoneDoc) && /No estimator formulas changed/.test(milestoneDoc)],
  ["backlog updated", /Now Wired In The Default Native Shell/.test(backlog) && /Still Intentionally Absent Or Deferred/.test(backlog)],
  ["no stale fake checksum", !/1A5C7D9B3E4F2C6D8A9B0C1D2E3F4A5B6C7D8E9F0A1B2C3D4E5F6A7B8C9D0E1/.test(app)],
  ["no stale v2.0 build string", !/2\.0\.0 \(Build 2025\.05\.12\.1015\)/.test(app)],
  ["no mojibake", !/RÂ²/.test(app + milestoneDoc + backlog)],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  milestone: "v2_45_0_mockup_visible_feature_completion",
  passed: failures.length === 0,
  checks: Object.fromEntries(checks),
  failures,
};

const outputPath = path.join(root, "validation", "results", "v245_mockup_feature_completion_audit.json");
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));

if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}

console.log(JSON.stringify(result, null, 2));
