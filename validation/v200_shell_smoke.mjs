import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const app = read("src/App.tsx");
const nav = read("src/components/NavRail.tsx");
const trust = read("src/components/TrustCenterWorkspace.tsx");
const settings = read("src/components/SettingsWorkspace.tsx");
const styles = read("src/styles.css");
const types = read("src/types.ts");
const topbar = read("src/components/TopBar.tsx");
const packageJson = JSON.parse(read("package.json"));

const checks = [
  ["package version is 2.0.0", packageJson.version === "2.0.0"],
  ["artifact label is v2.0.0", packageJson.scripts["qpls:release:artifacts"].includes("v2_0_0_design_system_and_shell")],
  ["workspace type includes Trust and Settings", types.includes('"trust"') && types.includes('"settings"')],
  ["nav rail exposes Trust and Settings", nav.includes('label: "Trust"') && nav.includes('label: "Settings"')],
  ["app routes Trust and Settings workspaces", app.includes("<TrustCenterWorkspace />") && app.includes("<SettingsWorkspace />")],
  ["smoke API allows v2 workspace routes", app.includes('"trust"') && app.includes('"settings"')],
  ["top bar shows v2 shell mark", topbar.includes("v2.0.0 design system shell")],
  ["Trust Center has scope language", trust.includes("Validation policy") && trust.includes("No SmartPLS equivalence")],
  ["Settings has UI-only boundary", settings.includes("not statistical results") && settings.includes("threshold colors")],
  ["v2 design tokens exist", ["--q2-bg", "--q2-panel", "--q2-teal", "qpls2-workspace"].every((token) => styles.includes(token))],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_0_0_design_system_and_shell",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v200_shell_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.0.0 shell smoke passed");
