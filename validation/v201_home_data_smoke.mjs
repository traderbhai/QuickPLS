import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const packageJson = JSON.parse(read("package.json"));
const home = read("src/components/OnboardingWorkspace.tsx");
const data = read("src/components/DataWorkspace.tsx");
const styles = read("src/styles.css");
const topbar = read("src/components/TopBar.tsx");

const checks = [
  ["package version is 2.0.1", packageJson.version === "2.0.1"],
  ["artifact label is v2.0.1", packageJson.scripts["qpls:release:artifacts"].includes("v2_0_1_home_data_redesign")],
  ["top bar shows v2.0.1 milestone", topbar.includes("v2.0.1 home and data redesign")],
  ["Home uses v2 workspace class", home.includes("qpls2-workspace home-v2-workspace")],
  ["Home has current workspace hero", home.includes("home-v2-hero") && home.includes("Current workspace")],
  ["Home keeps primary project commands wired", ["quickpls:save-project", "quickpls:open-project", "quickpls:open-demo-project"].every((token) => home.includes(token))],
  ["Home has guided dataset workflow", home.includes("guided-research-flow home-v2-guide")],
  ["Data uses v2 workspace class", data.includes("qpls2-workspace data-page data-v2-workspace")],
  ["Data has import and quality top grid", data.includes("data-v2-top-grid") && data.includes("data-v2-import") && data.includes("data-v2-quality")],
  ["Data retains method guidance and prefix bridge", data.includes("What can I do with this data?") && data.includes("Create Constructs From Prefixes")],
  ["Data keeps preview and metadata workbench", data.includes("data-workbench") && data.includes("Column profile")],
  ["v2 Home/Data CSS exists", ["home-v2-hero", "home-v2-command-grid", "data-v2-top-grid", "data-v2-workspace"].every((token) => styles.includes(token))],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_0_1_home_data_redesign",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v201_home_data_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.0.1 Home/Data smoke passed");
