import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const root = process.cwd();
const resultPath = join(root, "validation", "results", "v2250_model_workbench_smoke.json");

function read(relativePath) {
  return readFileSync(join(root, relativePath), "utf8");
}

function check(name, passed, evidence) {
  return { name, passed: Boolean(passed), evidence };
}

const app = read("src/App.tsx");
const canvas = read("src/components/ModelCanvas.tsx");
const explorer = read("src/components/Explorer.tsx");
const inspector = read("src/components/Inspector.tsx");
const issuesPaneExists = existsSync(join(root, "src", "components", "ModelIssuesPane.tsx"));
const issuesPane = issuesPaneExists ? read("src/components/ModelIssuesPane.tsx") : "";
const styles = read("src/styles.css");

const checks = [
  check("Model issues/output pane exists", issuesPaneExists, "src/components/ModelIssuesPane.tsx"),
  check("App renders model issues/output pane", app.includes("<ModelIssuesPane />"), "Model route composes Explorer, ModelCanvas, Inspector, and ModelIssuesPane."),
  check("Explorer has native workbench marker", explorer.includes('data-v225-model-workbench="explorer-tree"'), "Explorer remains the model inventory tree."),
  check("Canvas has native workbench marker", canvas.includes('data-v225-model-workbench="sem-canvas"') && canvas.includes("model-v225-canvas"), "Existing React Flow SEM canvas is marked for v2.25 styling."),
  check("Inspector has property-sheet marker", inspector.includes('data-v225-model-workbench="property-inspector"'), "Right inspector remains contextual and property-sheet styled."),
  check("Bottom pane has issue/output marker", issuesPane.includes('data-v225-model-workbench="issues-output-pane"'), "Bottom pane exposes model issues, selection, publication check, and actions."),
  check("Bottom pane uses existing readiness logic", issuesPane.includes("analysisReadiness") && issuesPane.includes("validate") === false, "Pane consumes frontend readiness state without changing engine logic."),
  check("Bottom pane provides publication check", issuesPane.includes("Publication check") && issuesPane.includes("off-canvas"), "Publication check flags layout/report defects."),
  check("Workbench action preserves SmartPLS arrange", issuesPane.includes('autoLayout("smartpls")'), "Arrange action calls existing layout command."),
  check("Focus Diagram hides side panes", styles.includes(".focus-diagram-mode .model-v225-explorer") && styles.includes(".focus-diagram-mode .model-v225-bottom-pane"), "Focus mode suppresses chrome while keeping the canvas."),
  check("Three-row model workbench grid exists", styles.includes("grid-template-rows: auto minmax(0, 1fr) 112px"), "Model shell includes workflow band, canvas row, and output pane."),
  check("No mojibake in touched model workbench files", ![canvas, explorer, inspector, issuesPane, styles].some((source) => source.includes("RÂ²")), "Touched model workbench sources render R-squared text correctly."),
];

const passed = checks.every((item) => item.passed);
mkdirSync(dirname(resultPath), { recursive: true });
writeFileSync(resultPath, JSON.stringify({
  id: "v2250_model_workbench_smoke",
  milestone: "v2_25_0_model_workbench_integration",
  passed,
  generated_at: new Date().toISOString(),
  checks,
}, null, 2));

if (!passed) {
  console.error(JSON.stringify({ passed, failed: checks.filter((item) => !item.passed) }, null, 2));
  process.exit(1);
}

console.log(`v2.25 model workbench smoke passed: ${resultPath}`);
