import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultPath = path.join(root, "validation", "results", "v2180_model_run_results_smoke.json");

const checks = [];
function check(name, passed, detail) {
  checks.push({ name, passed: Boolean(passed), detail });
}

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

const modelCanvas = read("src/components/ModelCanvas.tsx");
const runWorkspace = read("src/components/RunWorkspace.tsx");
const runHistory = read("src/components/RunHistory.tsx");
const styles = read("src/styles.css");

check(
  "Model screen marker",
  modelCanvas.includes('data-v218-mockup-screen="model"') && modelCanvas.includes("model-v218-canvas-shell"),
  "ModelCanvas has v2.18 marker and shell class while preserving existing canvas component."
);
check(
  "Model toolbar marker",
  modelCanvas.includes("model-v218-toolbar"),
  "Model toolbar has v2.18 mockup alignment class."
);
check(
  "Run screen marker",
  runWorkspace.includes('data-v218-mockup-screen="run"') && runWorkspace.includes("run-v218-workspace"),
  "RunWorkspace has v2.18 screen marker and workspace class."
);
check(
  "Results screen marker",
  runHistory.includes('data-v218-mockup-screen="results"') && runHistory.includes("results-v218-workspace"),
  "RunHistory has v2.18 results marker and workspace class."
);
check(
  "Empty results marker",
  runHistory.includes('data-v218-mockup-screen="results-empty"'),
  "Empty results state is covered by v2.18 marker."
);
check(
  "v2.18 CSS hooks",
  [".model-v218-canvas-shell", ".run-v218-workspace", ".results-v218-workspace"].every((selector) => styles.includes(selector)),
  "Styles include Model, Run, and Results mockup alignment hooks."
);
check(
  "R squared text is clean",
  ![modelCanvas, runWorkspace, runHistory, styles].some((source) => source.includes("RÂ²") || source.includes("RÃ")),
  "Touched UI sources do not contain mojibake for R²."
);
check(
  "No SmartPLS equivalence claim",
  ![modelCanvas, runWorkspace, runHistory, styles].some((source) => /SmartPLS equivalen/i.test(source)),
  "QuickPLS keeps SmartPLS-like visual wording without equivalence claims."
);

const passed = checks.every((entry) => entry.passed);
fs.mkdirSync(path.dirname(resultPath), { recursive: true });
fs.writeFileSync(resultPath, JSON.stringify({ passed, milestone: "v2_18_0_model_run_results_mockup_alignment", generatedAt: new Date().toISOString(), checks }, null, 2));

if (!passed) {
  console.error(JSON.stringify({ passed, failed: checks.filter((entry) => !entry.passed) }, null, 2));
  process.exit(1);
}

console.log(`v2.18 Model/Run/Results smoke passed: ${resultPath}`);
