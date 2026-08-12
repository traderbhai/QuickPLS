import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const sources = {
  commands: read("src/domain/desktopCommands.ts"),
  topbar: read("src/components/TopBar.tsx"),
  statusbar: read("src/components/StatusBar.tsx"),
  store: read("src/store.ts"),
  types: read("src/types.ts"),
  styles: read("src/styles.css"),
};

const requiredCommands = [
  "file.new",
  "file.open",
  "file.save",
  "data.import",
  "calculate.run",
  "calculate.cancel",
  "model.validate",
  "model.arrange",
  "results.copy_run_list",
  "results.export_table",
  "report.export",
  "tools.preferences",
  "help.shortcuts",
];

const checks = [
  ["command registry marker", sources.topbar.includes("data-v222-command-registry")],
  ["command registry file", sources.commands.includes("DESKTOP_COMMANDS") && sources.commands.includes("DESKTOP_MENU_ORDER")],
  ["required commands registered", requiredCommands.every((commandId) => sources.commands.includes(`id: "${commandId}"`))],
  ["native menu ordering includes tools/window", sources.commands.includes('"tools"') && sources.commands.includes('"window"')],
  ["dialog manager entrypoints", ["new_project", "open_project", "import_data", "export_options", "calculation_setup", "method_scope", "settings", "help_shortcuts"].every((id) => sources.types.includes(id) || sources.topbar.includes(id))],
  ["command feedback status", sources.topbar.includes("setDesktopCommandStatus") && sources.statusbar.includes("status-command-feedback")],
  ["disabled command reason surface", sources.commands.includes("requiresReasonWhenDisabled") && sources.styles.includes(".status-command-feedback")],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_22_0_menu_commands_dialogs_native_base",
  generated_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
};

fs.writeFileSync(path.join(outDir, "v2220_native_commands_smoke.json"), JSON.stringify(result, null, 2));
if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}
console.log("v2.22 native commands smoke passed");
