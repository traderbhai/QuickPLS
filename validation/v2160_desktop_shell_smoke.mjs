import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const topBar = read("src/components/TopBar.tsx");
const styles = read("src/styles.css");
const store = read("src/store.ts");
const types = read("src/types.ts");

const checks = [
  ["desktop shell marker", topBar.includes('data-v216-desktop-shell="menu-bar"')],
  ["all desktop menus", ["File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Help"].every((label) => topBar.includes(`label: "${label}"`))],
  ["desktop dialogs", ["new_project", "open_project", "import_data", "export_options", "calculation_setup", "method_scope", "settings", "help_shortcuts"].every((id) => topBar.includes(id))],
  ["dialog store state", store.includes("activeDesktopMenu") && store.includes("activeDesktopDialog")],
  ["dialog types", types.includes("DesktopMenuId") && types.includes("DesktopDialogId")],
  ["five-row app shell", styles.includes("grid-template-rows: 30px 28px 42px minmax(0, 1fr) 28px")],
  ["menu css", styles.includes(".desktop-menu-bar") && styles.includes(".desktop-menu-popover")],
  ["dialog css", styles.includes(".desktop-dialog-backdrop") && styles.includes(".desktop-dialog-titlebar")],
  ["rail section labels hidden", styles.includes(".nav-section-label { position: absolute")],
  ["no native tauri menu dependency", !topBar.includes("MenuItem") && !topBar.includes("@tauri-apps/api/menu")],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_16_0_desktop_shell_visual_contract",
  generated_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
};

fs.writeFileSync(path.join(outDir, "v2160_desktop_shell_smoke.json"), JSON.stringify(result, null, 2));
if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}
console.log("v2.16 desktop shell smoke passed");
