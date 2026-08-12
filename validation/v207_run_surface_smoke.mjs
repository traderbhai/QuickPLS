import fs from "node:fs";

const checks = [];
const add = (name, passed, detail) => checks.push({ name, passed, detail });
const read = (path) => fs.readFileSync(path, "utf8");

const run = read("src/components/RunWorkspace.tsx");
const styles = read("src/styles.css");
const pkg = JSON.parse(read("package.json"));
const topBar = read("src/components/TopBar.tsx");

const requiredRunTokens = [
  "run-v2-workspace",
  "run-v2-hero",
  "run-v2-readiness-panel",
  "run-v2-output-preview",
  "run-v2-execution-plan",
  "run-v2-handoff-grid",
  "Run disabled:",
  "Output preview",
  "Execution plan",
  "Open setup",
  "Open results",
  "Prepare report",
  "quickpls:run-analysis",
  "analysisReadiness",
  "effectiveMethodStatus",
];

for (const token of requiredRunTokens) {
  add(`Run workspace contains ${token}`, run.includes(token), token);
}

const requiredStyleTokens = [
  ".run-v2-workspace",
  ".run-v2-hero",
  ".run-v2-readiness-panel",
  ".run-v2-output-preview",
  ".run-v2-execution-plan",
  ".run-v2-handoff-grid",
];

for (const token of requiredStyleTokens) {
  add(`Styles contain ${token}`, styles.includes(token), token);
}

add("package version is 2.0.7", pkg.version === "2.0.7", pkg.version);
add("release artifact label is v2.0.7", pkg.scripts["qpls:release:artifacts"].includes("v2_0_7_run_execution_surface_redesign"), pkg.scripts["qpls:release:artifacts"]);
add("top bar label is current", topBar.includes("v2.0.7 run surface redesign"), "TopBar alpha mark");
add("Run source has no mojibake", !/[ÃÂ�ï¿½]/.test(run), "RunWorkspace encoding");

const passed = checks.every((check) => check.passed);
const result = {
  passed,
  milestone: "v2_0_7_run_execution_surface_redesign",
  generated_at: new Date().toISOString(),
  checks,
};

fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync("validation/results/v207_run_surface_smoke.json", JSON.stringify(result, null, 2));

if (!passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}

console.log("v2.0.7 Run surface smoke passed");
