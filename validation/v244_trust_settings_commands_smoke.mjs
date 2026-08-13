import fs from "node:fs";

const out = "validation/results/v244_trust_settings_commands_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");

const checks = [
  ["trust rows are generated from applicability/status data", adapter.includes("adaptTrustRows") && adapter.includes("evaluateMethodApplicability")],
  ["release integrity dialog calls desktop checksum verification", shell.includes("verifyReleaseChecksumsFromShell") && shell.includes("ReleaseIntegrityDialog")],
  ["settings screen reads and writes UI preferences", shell.includes("uiPreferences") && shell.includes("setUiPreferences")],
  ["settings status bar does not claim fake unsaved changes", !shell.includes("You have unsaved settings changes.")],
  ["status bar command feedback is wired", shell.includes("quickpls:status-message") && shell.includes("command-feedback")],
  ["global close guard is present", shell.includes("close_project") && shell.includes("saveAndCloseProject")],
  ["no visible default pause command", !shell.includes('label: "Pause"')],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 trust/settings/commands smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v244 trust/settings/commands smoke passed: ${out}`);
