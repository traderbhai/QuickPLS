import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const sources = {
  topbar: read("src/components/TopBar.tsx"),
  statusbar: read("src/components/StatusBar.tsx"),
  styles: read("src/styles.css"),
  types: read("src/types.ts"),
  store: read("src/store.ts"),
};

const checks = [
  ["native shell marker", sources.topbar.includes("data-v221-native-shell")],
  ["native menu set", ["File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Tools", "Window", "Help"].every((label) => sources.topbar.includes(`label: "${label}"`))],
  ["tools and window menus typed", sources.types.includes('"tools"') && sources.types.includes('"window"')],
  ["neutral desktop chrome styles", sources.styles.includes(".q2-desktop-command-strip") && sources.topbar.includes("q2-desktop-title")],
  ["bottom status bar retained", sources.statusbar.includes("status-bar") && sources.statusbar.includes("desktopCommandStatus")],
  ["ui-only state boundary", sources.store.includes("desktopCommandStatus") && !sources.store.includes("fingerprint")],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_21_0_desktop_design_system_shell",
  generated_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
};

fs.writeFileSync(path.join(outDir, "v2210_native_shell_smoke.json"), JSON.stringify(result, null, 2));
if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}
console.log("v2.21 native shell smoke passed");
