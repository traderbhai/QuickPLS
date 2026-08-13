import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultsDir = path.join(root, "validation", "results");
fs.mkdirSync(resultsDir, { recursive: true });

const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const readJson = (relative) => JSON.parse(read(relative));

const methodApplicability = read("src/domain/methodApplicability.ts");
const dataWorkspace = read("src/components/DataWorkspace.tsx");
const explorer = read("src/components/Explorer.tsx");
const analysisCatalog = read("src/components/AnalysisCatalog.tsx");
const topBar = read("src/components/TopBar.tsx");
const styles = read("src/styles.css");
const registry = readJson("validation/development_slices.json");

const source = [
  methodApplicability,
  dataWorkspace,
  explorer,
  analysisCatalog,
  topBar,
  styles,
].join("\n");

const checks = [
  ["registry current stage v2.15", registry.current_stage === "v2_15_0_workflow_method_guidance_triage_pass"],
  ["data workspace v2.15 marker", dataWorkspace.includes('data-workflow-method-guidance-triage="v2.15.0"')],
  ["data next move panel", dataWorkspace.includes("data-v215-next-move") && dataWorkspace.includes("data-data-guidance-next-action")],
  ["model explorer v2.15 marker", explorer.includes('data-workflow-method-guidance-triage="v2.15.0"')],
  ["model guidance action labels", explorer.includes("Guidance is based on construct modes") && explorer.includes("<em>{item.actionLabel}</em>")],
  ["setup v2.15 marker", analysisCatalog.includes('data-workflow-method-guidance-triage="v2.15.0"')],
  ["setup decision panel", analysisCatalog.includes("setup-v215-decision-panel") && analysisCatalog.includes("data-selected-method-next-action")],
  ["method cards expose next action", analysisCatalog.includes("data-method-next-action") && analysisCatalog.includes("data-method-failed-check")],
  ["expected method explanation", analysisCatalog.includes("If you expected another method")],
  ["topbar current guidance label", topBar.includes("v2.15.0 guided setup")],
  ["topbar conservative method picker", topBar.includes("More methods in Setup") && topBar.includes("data-topbar-guidance-count")],
  ["R squared renders correctly", methodApplicability.includes("R²") && !source.includes("RÂ²") && !source.includes("RÃ")],
  ["frontend only smoke boundary", !source.includes("F_ml =") && !source.includes("Sigma(theta)")],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_15_0_workflow_method_guidance_triage_pass",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(
  path.join(resultsDir, "v2150_workflow_method_guidance_smoke.json"),
  JSON.stringify(report, null, 2),
);

if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}

console.log("v2.15 workflow method guidance smoke passed");
