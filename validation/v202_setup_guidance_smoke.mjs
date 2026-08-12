import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const packageJson = JSON.parse(read("package.json"));
const setup = read("src/components/AnalysisCatalog.tsx");
const styles = read("src/styles.css");
const topbar = read("src/components/TopBar.tsx");

const requiredSetupTokens = [
  "qpls2-workspace setup-v2-workspace",
  "setup-v2-hero",
  "Selected calculation",
  "Why trust this method?",
  "setup-v2-readiness",
  "setup-v2-method-browser",
  "setup-v2-sidecar",
  "Requirement checks",
  "setup-v2-presets",
  "setup-v2-launch",
  "setup-v2-preview",
];

const requiredStyleTokens = [
  ".setup-v2-workspace",
  ".setup-v2-hero",
  ".setup-v2-main",
  ".setup-v2-method-browser",
  ".setup-v2-sidecar",
  ".setup-v2-requirement",
  ".setup-v2-presets",
  "@media (max-width: 1320px)",
];

const checks = [
  ["package version is 2.0.2", packageJson.version === "2.0.2"],
  ["artifact label is v2.0.2", packageJson.scripts["qpls:release:artifacts"].includes("v2_0_2_setup_method_guidance_redesign")],
  ["top bar shows v2.0.2 milestone", topbar.includes("v2.0.2 setup guidance redesign")],
  ["Setup has v2 information architecture", requiredSetupTokens.every((token) => setup.includes(token))],
  ["Setup keeps applicability-driven method sections", ["Recommended for this project", "Available after setup", "Advanced diagnostics", "Standalone analyses"].every((token) => setup.includes(token))],
  ["Setup has selected-method requirement actions", ["check.actionView", "check.actionLabel", "setView(check.actionView)"].every((token) => setup.includes(token))],
  ["Setup no longer imports ActionStrip", !setup.includes("ActionStrip")],
  ["Setup output text uses R²", setup.includes("R²") && !setup.includes("RÂ²")],
  ["v2 Setup CSS exists", requiredStyleTokens.every((token) => styles.includes(token))],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_0_2_setup_method_guidance_redesign",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v202_setup_guidance_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.0.2 Setup guidance smoke passed");
