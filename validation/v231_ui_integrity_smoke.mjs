import path from "node:path";
import {
  RESULTS,
  collectV2ShellSnapshot,
  ensureDir,
  evaluateV2ShellIntegrity,
  issuesFromChecklist,
  withPreviewPage,
  writeJson,
} from "./lib/v2_ui_smoke_harness.mjs";

const ARTIFACTS = path.join(RESULTS, "screens", "v231", "ui-integrity");
const OUTPUT = path.join(RESULTS, "v231_ui_integrity_smoke.json");
const PORT = 53231;

await ensureDir(ARTIFACTS);

const result = await withPreviewPage({
  port: PORT,
  run: async ({ page, errors }) => {
    await page.screenshot({ path: path.join(ARTIFACTS, "01_v2_shell_integrity.png"), fullPage: true });
    const snapshot = await collectV2ShellSnapshot(page);
    const checklist = {
      ...evaluateV2ShellIntegrity(snapshot, "v2.3.1 UI integrity consolidation"),
      no_console_errors: errors.length === 0,
    };
    const issues = issuesFromChecklist(checklist);
    return {
      schema_version: 1,
      target: "QuickPLS v2.3.1 UI integrity consolidation smoke",
      passed: issues.length === 0,
      generated_at: new Date().toISOString(),
      checklist,
      snapshot: { ...snapshot, body: undefined },
      issues,
      errors,
      screenshots_dir: ARTIFACTS,
    };
  },
});

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
