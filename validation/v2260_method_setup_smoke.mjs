import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const root = process.cwd();
const resultPath = join(root, "validation", "results", "v2260_method_setup_smoke.json");

function read(relativePath) {
  return readFileSync(join(root, relativePath), "utf8");
}

function check(name, passed, evidence) {
  return { name, passed: Boolean(passed), evidence };
}

const catalog = read("src/components/AnalysisCatalog.tsx");
const applicability = read("src/domain/methodApplicability.ts");
const topBar = read("src/components/TopBar.tsx");
const styles = read("src/styles.css");
const goodRSquared = String.fromCharCode(82, 178);
const badRSquared = String.fromCharCode(82, 194, 178);

const checks = [
  check("Setup has v2.26 calculation-center marker", catalog.includes('data-v226-method-setup-center="true"') && catalog.includes("setup-v226-workspace"), "src/components/AnalysisCatalog.tsx"),
  check("Setup page uses native calculation wording", catalog.includes('title="Calculation Setup"') && catalog.includes("Selected calculation command"), "Setup title and selected calculation header are desktop setup wording."),
  check("Method category strip exists", catalog.includes("setup-v226-category-tabs") && catalog.includes("Not applicable"), "Category strip covers method applicability states."),
  check("Recommended and available lanes are explicit", catalog.includes('title="Recommended for this project"') && catalog.includes('title="Available now"') && catalog.includes('title="Available with setup"'), "Primary method lanes are separate."),
  check("Scoped-out methods remain visible with reasons", catalog.includes('title="Not applicable or scoped out"') && catalog.includes("Methods stay visible with exact reasons"), "Unavailable methods are not silently hidden."),
  check("Inference add-ons are separated from primary methods", catalog.includes("setup-v226-addons") && catalog.includes("inferenceAddOns") && catalog.includes("Inference add-ons"), "Bootstrap is configured as an add-on panel."),
  check("Bootstrap remains an add-on setting", catalog.includes("settings.bootstrapSamples > 0") && catalog.includes("Enable") && catalog.includes("Freedman-Lane permutation"), "Bootstrap/permutation are shown as add-on and expert concepts."),
  check("Selected method requirements remain action-oriented", catalog.includes("Requirement checks") && catalog.includes("First blocker") && catalog.includes("actionLabel"), "Requirement checks expose exact reasons and actions."),
  check("Top-bar method selector stays conservative", topBar.includes("topBarMethods") && topBar.includes("More methods in Setup"), "TopBar sends broad method discovery to Setup."),
  check("Applicability engine groups method categories", applicability.includes("core_model_estimation") && applicability.includes("standalone_analysis") && applicability.includes("prediction_segmentation"), "Method categories remain available to the Setup center."),
  check("R-squared output labels render correctly", applicability.includes(goodRSquared) && !applicability.includes(badRSquared) && !catalog.includes(badRSquared), "No stale R-squared mojibake in Setup/applicability sources."),
  check("v2.26 desktop CSS exists", styles.includes(".setup-v226-workspace") && styles.includes(".setup-v226-category-tabs") && styles.includes(".setup-v226-addons"), "src/styles.css contains v2.26 layout rules."),
  check("Setup stays frontend-only", !catalog.includes("invoke(") && !applicability.includes("invoke("), "No Tauri/backend command calls added to Setup or applicability logic."),
];

const passed = checks.every((item) => item.passed);
mkdirSync(dirname(resultPath), { recursive: true });
writeFileSync(resultPath, JSON.stringify({
  id: "v2260_method_setup_smoke",
  milestone: "v2_26_0_method_setup_applicability_center",
  passed,
  generated_at: new Date().toISOString(),
  checks,
}, null, 2));

if (!passed) {
  console.error(JSON.stringify({ passed, failed: checks.filter((item) => !item.passed) }, null, 2));
  process.exit(1);
}

console.log(`v2.26 method setup smoke passed: ${resultPath}`);
