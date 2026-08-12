import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const home = read("src/components/OnboardingWorkspace.tsx");
const data = read("src/components/DataWorkspace.tsx");
const setup = read("src/components/AnalysisCatalog.tsx");
const styles = read("src/styles.css");

const checks = [
  ["home screen marker", home.includes('data-v217-mockup-screen="home"')],
  ["data screen marker", data.includes('data-v217-mockup-screen="data"')],
  ["setup screen marker", setup.includes('data-v217-mockup-screen="setup"')],
  ["home launcher alignment", home.includes("home-v217-workspace") && home.includes("home-v217-launcher")],
  ["data workbench alignment", data.includes("data-v217-overview") && data.includes("data-v217-preview")],
  ["setup method alignment", setup.includes("setup-v217-method-summary") && setup.includes("setup-v217-main")],
  ["mockup css classes", [".home-v217-workspace", ".data-v217-workspace", ".setup-v217-workspace"].every((token) => styles.includes(token))],
  ["dense home css", styles.includes(".home-v217-workspace .home-v2-command-grid") && styles.includes(".home-v217-workspace .home-v2-current")],
  ["dense data css", styles.includes(".data-v217-preview .data-workbench") && styles.includes(".data-v217-workspace .data-quality-grid")],
  ["dense setup css", styles.includes(".setup-v217-workspace .method-guidance-grid") && styles.includes(".setup-v217-workspace .setup-v2-status-strip")],
  ["frontend-only", ![home, data, setup, styles].join("\n").includes("F_ml =")],
  ["no mojibake", ![home, data, setup, styles].join("\n").includes("RÃ‚Â²")],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_17_0_home_data_setup_mockup_alignment",
  generated_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
};

fs.writeFileSync(path.join(outDir, "v2170_home_data_setup_smoke.json"), JSON.stringify(result, null, 2));
if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}
console.log("v2.17 Home/Data/Setup smoke passed");
