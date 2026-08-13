import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
const resultPath = path.join(outDir, "v2200_mockup_parity_smoke.json");
fs.mkdirSync(outDir, { recursive: true });

const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const checks = [];
const check = (name, passed, detail) => checks.push({ name, passed: Boolean(passed), detail });

const sources = {
  topbar: read("src/components/TopBar.tsx"),
  home: read("src/components/OnboardingWorkspace.tsx"),
  data: read("src/components/DataWorkspace.tsx"),
  setup: read("src/components/AnalysisCatalog.tsx"),
  model: read("src/components/ModelCanvas.tsx"),
  run: read("src/components/RunWorkspace.tsx"),
  results: read("src/components/RunHistory.tsx"),
  report: read("src/components/ReportsWorkspace.tsx"),
  trust: read("src/components/TrustCenterWorkspace.tsx"),
  settings: read("src/components/SettingsWorkspace.tsx"),
  store: read("src/store.ts"),
  types: read("src/types.ts"),
  styles: read("src/styles.css"),
};

check(
  "Desktop shell",
  sources.topbar.includes('data-v216-desktop-shell="menu-bar"') &&
    ["File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Help"].every((label) => sources.topbar.includes(`label: "${label}"`)),
  "React desktop menu bar and command shell are present."
);
check(
  "Desktop dialogs",
  ["new_project", "open_project", "import_data", "export_options", "calculation_setup", "method_scope", "settings", "help_shortcuts"].every((id) => sources.topbar.includes(id)) &&
    sources.store.includes("activeDesktopDialog") &&
    sources.types.includes("DesktopDialogId"),
  "Frontend-only desktop dialog manager state and dialog ids are present."
);
check("Home screen", sources.home.includes('data-v217-mockup-screen="home"') && sources.home.includes("home-v217-workspace"), "Home launcher uses the v2.17 mockup alignment.");
check("Data screen", sources.data.includes('data-v217-mockup-screen="data"') && sources.data.includes("data-v217-workspace"), "Data workspace uses the v2.17 mockup alignment.");
check("Setup screen", sources.setup.includes('data-v217-mockup-screen="setup"') && sources.setup.includes("setup-v217-workspace"), "Setup workspace uses the v2.17 mockup alignment.");
check("Model screen", sources.model.includes('data-v218-mockup-screen="model"') && sources.model.includes("model-v218-canvas-shell"), "Model shell uses the v2.18 mockup alignment.");
check("Run screen", sources.run.includes('data-v218-mockup-screen="run"') && sources.run.includes("run-v218-workspace"), "Run workspace uses the v2.18 mockup alignment.");
check("Results screen", sources.results.includes('data-v218-mockup-screen="results"') && sources.results.includes("results-v218-workspace"), "Results workspace uses the v2.18 mockup alignment.");
check("Report screen", sources.report.includes('data-v219-mockup-screen="report"') && sources.report.includes("report-v219-workspace"), "Report workspace uses the v2.19 mockup alignment.");
check("Trust Center screen", sources.trust.includes('data-v219-mockup-screen="trust"') && sources.trust.includes("trust-v219-workspace"), "Trust Center uses the v2.19 mockup alignment.");
check("Settings screen", sources.settings.includes('data-v219-mockup-screen="settings"') && sources.settings.includes("settings-v219-workspace"), "Settings uses the v2.19 mockup alignment.");
check(
  "Shared visual contract",
  [".app-shell", ".desktop-menu-bar", ".home-v217-workspace", ".model-v218-canvas-shell", ".report-v219-workspace"].every((token) => sources.styles.includes(token)),
  "Shared desktop shell and screen-specific mockup CSS hooks are present."
);
check(
  "No known text corruption",
  !Object.values(sources).some((source) => source.includes("RÃ") || source.includes("RÂ") || source.includes("Validation fixture")),
  "Mockup-aligned sources avoid R-squared mojibake and stale validation-fixture copy."
);
check(
  "No SmartPLS equivalence claim",
  !Object.values(sources).some((source) => /SmartPLS[- ]equivalent|equivalent to SmartPLS/i.test(source)),
  "Visual inspiration does not become a SmartPLS equivalence claim."
);
check(
  "Frontend-only boundary",
  !Object.values(sources).some((source) => source.includes("F_ml =") || source.includes("crates/qpls-estimation")),
  "The parity pass stays in frontend/product surfaces."
);

const passed = checks.every((entry) => entry.passed);
fs.writeFileSync(resultPath, JSON.stringify({ passed, milestone: "v2_20_0_quickpls_2_mockup_parity_release_audit", generatedAt: new Date().toISOString(), checks }, null, 2));

if (!passed) {
  console.error(JSON.stringify({ passed, failed: checks.filter((entry) => !entry.passed) }, null, 2));
  process.exit(1);
}

console.log(`v2.20 mockup parity smoke passed: ${resultPath}`);
