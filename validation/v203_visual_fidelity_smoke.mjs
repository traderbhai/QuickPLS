import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const packageJson = JSON.parse(read("package.json"));
const styles = read("src/styles.css");
const topbar = read("src/components/TopBar.tsx");
const home = read("src/components/OnboardingWorkspace.tsx");
const data = read("src/components/DataWorkspace.tsx");
const setup = read("src/components/AnalysisCatalog.tsx");
const results = read("src/components/RunHistory.tsx");
const contract = read("docs/V2_UI_VISUAL_CONTRACT.md");

const requiredTokens = [
  "--q2-page-gutter-x",
  "--q2-page-gutter-y",
  "--q2-panel-pad",
  "--q2-section-gap",
  "--q2-control-height",
  "--q2-toolbar-height",
  "--q2-shadow-soft",
  "--q2-ink",
  "--q2-danger",
];

const requiredPrimitives = [
  ".qpls2-workspace",
  ".qpls2-panel",
  ".qpls2-page-title",
  ".qpls2-page-subtitle",
  ".qpls2-command-row",
  ".qpls2-card-title",
  ".qpls2-card-body",
  ".qpls2-chip",
  ".qpls2-primary-action",
  ".qpls2-secondary-action",
];

const requiredResultsV2 = [
  "results-v2-workspace",
  "results-v2-command-center",
  "results-v2-run-hero",
  "results-v2-summary-row",
  "results-v2-selected-run-list",
  "results-v2-table-shell",
];

const checks = [
  ["package version is 2.0.3", packageJson.version === "2.0.3"],
  ["artifact label is v2.0.3", packageJson.scripts["qpls:release:artifacts"].includes("v2_0_3_visual_fidelity_foundation")],
  ["top bar shows v2.0.3 milestone", topbar.includes("v2.0.3 visual fidelity foundation")],
  ["visual contract exists and names desktop targets", contract.includes("1440x900") && contract.includes("1280x800")],
  ["visual contract preserves numerical boundary", contract.includes("does not change statistical engines")],
  ["required q2 tokens exist", requiredTokens.every((token) => styles.includes(token))],
  ["required qpls2 primitives exist", requiredPrimitives.every((token) => styles.includes(token))],
  ["Home/Data/Setup use qpls2 workspace", [home, data, setup].every((source) => source.includes("qpls2-workspace"))],
  ["Results v2 classes are present in component", requiredResultsV2.every((token) => results.includes(token))],
  ["Results v2 classes are styled", requiredResultsV2.every((token) => styles.includes(`.${token}`))],
  ["no mojibake markers in v2 surfaces", ![styles, topbar, home, data, setup, results, contract].join("\n").includes("RÃ")],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_0_3_visual_fidelity_foundation",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v203_visual_fidelity_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.0.3 visual fidelity smoke passed");
