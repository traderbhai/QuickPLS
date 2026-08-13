import fs from "node:fs";

const MILESTONE = "v2_0_9_mockup_fidelity_system";
const VERSION = "2.0.9";
const checks = [];
const add = (name, passed, detail) => checks.push({ name, passed: Boolean(passed), detail });
const read = (path) => fs.readFileSync(path, "utf8");

const pkg = JSON.parse(read("package.json"));
const contract = read("docs/V2_UI_VISUAL_CONTRACT.md");
const styles = read("src/styles.css");
const ui = read("src/components/Ui.tsx");
const topBar = read("src/components/TopBar.tsx");
const app = read("src/App.tsx");

const requiredContractTokens = [
  "Contract Status",
  "Mockup-Matching Rules",
  "Screen Completion Checklist",
  "1440x900",
  "1280x800",
  "D:\\QuickPLS\\target\\release\\artifacts",
  "shared `qpls2` primitives",
  "No stale version label",
];

for (const token of requiredContractTokens) {
  add(`visual contract includes ${token}`, contract.includes(token), token);
}

const requiredStyleTokens = [
  "--q2-bg",
  "--q2-panel",
  "--q2-teal",
  "--q2-page-gutter-x",
  ".qpls2-workspace",
  ".qpls2-panel",
  ".qpls2-primary-action",
  ".qpls2-secondary-action",
];

for (const token of requiredStyleTokens) {
  add(`styles include ${token}`, styles.includes(token), token);
}

const requiredUiTokens = [
  "PageHeader",
  "StatusBadge",
  "MethodScopeDrawer",
  "MethodConfidencePanel",
  "ReportabilityChecklist",
  "ResearchTable",
];

for (const token of requiredUiTokens) {
  add(`shared UI exposes ${token}`, ui.includes(token), token);
}

const requiredWorkspaceFiles = [
  "src/components/DataWorkspace.tsx",
  "src/components/AnalysisCatalog.tsx",
  "src/components/ModelCanvas.tsx",
  "src/components/RunWorkspace.tsx",
  "src/components/RunHistory.tsx",
  "src/components/ReportsWorkspace.tsx",
  "src/components/TrustCenterWorkspace.tsx",
  "src/components/SettingsWorkspace.tsx",
];

for (const path of requiredWorkspaceFiles) {
  add(`workspace source exists ${path}`, fs.existsSync(path), path);
}

add("package version is 2.0.9", pkg.version === VERSION, pkg.version);
add("release artifact label is v2.0.9", pkg.scripts["qpls:release:artifacts"].includes(MILESTONE), pkg.scripts["qpls:release:artifacts"]);
add("top bar label is current", topBar.includes("v2.0.9 mockup fidelity"), "TopBar alpha mark");
add("app applies desktop shell classes", app.includes("app-shell") && app.includes("workspace-shell") && app.includes("page-host"), "App shell");

const searchableSource = [
  contract,
  styles,
  ui,
  topBar,
  app,
  ...requiredWorkspaceFiles.filter((path) => fs.existsSync(path)).map(read),
].join("\n");

add("source has no mojibake", !/[ÃƒÆ’Ãƒâ€šÃ¯Â¿Â½ÃƒÂ¯Ã‚Â¿Ã‚Â½]|RÃ‚Â²/.test(searchableSource), "encoding");
add("source avoids SmartPLS equivalence claims", !/identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS/i.test(searchableSource), "claim boundary");

const passed = checks.every((check) => check.passed);
const result = {
  passed,
  milestone: MILESTONE,
  generated_at: new Date().toISOString(),
  checks,
};

fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync("validation/results/v209_mockup_fidelity_smoke.json", JSON.stringify(result, null, 2));

if (!passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}

console.log("v2.0.9 mockup fidelity smoke passed");
