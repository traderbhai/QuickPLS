import fs from "node:fs/promises";
import path from "node:path";
import { ROOT, RESULTS, ensureDir, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const OUTPUT = path.join(RESULTS, "v241_release_readiness_smoke.json");
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
const requiredScripts = [
  "qpls:v232:harness",
  "qpls:v240:public-docs",
  "qpls:v241:release-readiness",
  "qpls:desktop:build-versioned",
];
const requiredMilestoneDocs = [
  "docs/V2_UI_VISUAL_CONTRACT.md",
  "docs/V2_3_2_SHARED_UI_VERIFICATION_HARNESS.md",
  "docs/V2_4_0_PUBLIC_DOCUMENTATION_SCREENSHOT_REFRESH.md",
  "docs/V2_4_1_QUICKPLS_2_RELEASE_READINESS_AUDIT.md",
  "docs/RELEASE_NOTES_V2_4_0.md",
];

await ensureDir(RESULTS);

const readJson = async (relative) => JSON.parse(await fs.readFile(path.join(ROOT, relative), "utf8"));
const readText = async (relative) => fs.readFile(path.join(ROOT, relative), "utf8");

const [pkg, registry, readme] = await Promise.all([
  readJson("package.json"),
  readJson("validation/development_slices.json"),
  readText("README.md"),
]);
const screenshotStats = await Promise.all(requiredScreenshots.map(async (name) => {
  const file = path.join(SCREENSHOTS, name);
  const stat = await fs.stat(file).catch(() => null);
  return { name, exists: Boolean(stat), bytes: stat?.size ?? 0, referenced: readme.includes(`docs/screenshots/v2/${name}`) };
}));
const docStats = await Promise.all(requiredMilestoneDocs.map(async (name) => {
  const stat = await fs.stat(path.join(ROOT, name)).catch(() => null);
  return { name, exists: Boolean(stat), bytes: stat?.size ?? 0 };
}));
const latestSlices = registry.slices
  .filter((slice) => slice.id.startsWith("v2_"))
  .slice(-6)
  .map((slice) => ({ id: slice.id, status: slice.status, open: slice.gates.filter((gate) => gate.status === "open" || gate.status === "blocked").length }));

const checklist = {
  current_stage_is_v241: registry.current_stage === "v2_4_1_quickpls_2_release_readiness_audit",
  package_version_is_241: pkg.version === "2.4.1",
  required_scripts_exist: requiredScripts.every((script) => Object.hasOwn(pkg.scripts, script)),
  required_v2_docs_exist: docStats.every((item) => item.exists && item.bytes > 500),
  required_screenshots_exist: screenshotStats.every((item) => item.exists && item.bytes > 10_000),
  readme_references_core_screenshots: screenshotStats.filter((item) => item.name !== "settings.png").every((item) => item.referenced),
  latest_v2_slices_are_validated: latestSlices.length >= 6 && latestSlices.every((slice) => slice.status === "validated" && slice.open === 0),
  artifact_command_uses_v241_label: pkg.scripts["qpls:release:artifacts"]?.includes("v2_4_1_quickpls_2_release_readiness_audit") ?? false,
};
const failed = Object.entries(checklist).filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  schema_version: 1,
  target: "QuickPLS v2.4.1 release readiness smoke",
  passed: failed.length === 0,
  generated_at: new Date().toISOString(),
  checklist,
  screenshots: screenshotStats,
  docs: docStats,
  latest_v2_slices: latestSlices,
  failed,
};
await writeJson(OUTPUT, result);
console.log(JSON.stringify(result, null, 2));
if (!result.passed) process.exitCode = 1;
