import fs from "node:fs/promises";
import path from "node:path";
import { ROOT, RESULTS, ensureDir, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const OUTPUT = path.join(RESULTS, "v240_public_docs_smoke.json");
const SCREENSHOTS = path.join(ROOT, "docs", "screenshots", "v2");
const requiredScreenshots = [
  "home.png",
  "data-workspace.png",
  "sem-designer.png",
  "setup-guided-methods.png",
  "run-workspace.png",
  "results-workspace.png",
  "report-workspace.png",
  "trust-center.png",
  "settings.png",
];

await ensureDir(RESULTS);

const readme = await fs.readFile(path.join(ROOT, "README.md"), "utf8");
const stats = await Promise.all(requiredScreenshots.map(async (name) => {
  const file = path.join(SCREENSHOTS, name);
  const stat = await fs.stat(file).catch(() => null);
  return { name, exists: Boolean(stat), bytes: stat?.size ?? 0, referenced: readme.includes(`docs/screenshots/v2/${name}`) };
}));

const checklist = {
  readme_uses_v2_current_release: readme.includes("Current development release: `v2.4.0`"),
  readme_uses_generic_artifact_pattern: readme.includes("QuickPLS_<version>_<milestone>_<timestamp>_x64_setup.exe"),
  readme_links_v2_visual_contract: readme.includes("docs/V2_UI_VISUAL_CONTRACT.md"),
  required_screenshots_exist: stats.every((item) => item.exists && item.bytes > 10_000),
  readme_references_core_screenshots: stats.filter((item) => item.name !== "settings.png").every((item) => item.referenced),
};
const failed = Object.entries(checklist).filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  schema_version: 1,
  target: "QuickPLS v2.4.0 public documentation screenshot refresh smoke",
  passed: failed.length === 0,
  generated_at: new Date().toISOString(),
  checklist,
  screenshots: stats,
  failed,
};
await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
