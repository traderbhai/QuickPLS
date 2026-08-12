import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const home = read("src/components/OnboardingWorkspace.tsx");
const styles = read("src/styles.css");

const checks = [
  ["v2.23 marker", home.includes("data-v223-project-manager")],
  ["desktop start center", home.includes("home-v223-start-center") && home.includes("Project launcher")],
  ["current workspace summary", home.includes("Current workspace") && home.includes("Project summary")],
  ["recent project list", home.includes("Recent projects") && home.includes("recentProjectCards")],
  ["recovery autosave panel", home.includes("Recovery and autosave") && home.includes("autosave and recovery checks")],
  ["quick links", home.includes("Quick links") && home.includes("Trust Center") && home.includes("Keyboard shortcuts")],
  ["dense list detail css", styles.includes(".home-v223-manager-grid") && styles.includes(".home-v223-recent-row")],
  ["no marketing hero language", !/desktop-first workflow|Welcome to QuickPLS 2\\.0/i.test(home)],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_23_0_home_project_manager",
  generated_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
};

fs.writeFileSync(path.join(outDir, "v2230_home_project_manager_smoke.json"), JSON.stringify(result, null, 2));
if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}
console.log("v2.23 home project manager smoke passed");
