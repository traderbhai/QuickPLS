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

const ARTIFACTS = path.join(RESULTS, "screens", "v232", "shared-ui-harness");
const OUTPUT = path.join(RESULTS, "v232_shared_ui_harness_smoke.json");
const PORT = 53232;

await ensureDir(ARTIFACTS);

const result = await withPreviewPage({
  port: PORT,
  run: async ({ page, errors }) => {
    const initial = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "01_shell_before_blocker_navigation.png"), fullPage: true });

    await page.locator(".command-blocker-chip").click();
    await page.waitForTimeout(250);
    const afterBlockerClick = await collectV2ShellSnapshot(page);
    await page.screenshot({ path: path.join(ARTIFACTS, "02_shell_after_blocker_navigation.png"), fullPage: true });

    const checklist = {
      ...evaluateV2ShellIntegrity(initial, "v2.3.2 shared UI verification harness"),
      blocker_navigation_updates_destination_context:
        afterBlockerClick.view === "models" &&
        afterBlockerClick.destinationContext?.coachId === "top-command-bar" &&
        String(afterBlockerClick.destinationContext?.actionLabel ?? "").startsWith("Run blocker:"),
      shared_snapshot_exposes_command_contract:
        Boolean(initial.commandMethod) &&
        initial.commandState === "blocked" &&
        initial.runState === "blocked" &&
        initial.blockerAction.length > 0,
      no_console_errors: errors.length === 0,
    };
    const issues = issuesFromChecklist(checklist);
    return {
      schema_version: 1,
      target: "QuickPLS v2.3.2 shared UI verification harness smoke",
      passed: issues.length === 0,
      generated_at: new Date().toISOString(),
      checklist,
      snapshot: { ...initial, body: undefined },
      after_blocker_click: { ...afterBlockerClick, body: undefined },
      issues,
      errors,
      screenshots_dir: ARTIFACTS,
    };
  },
});

await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
