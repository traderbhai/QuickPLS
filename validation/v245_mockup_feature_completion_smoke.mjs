import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const appPath = path.join(root, "src", "v2", "NativePrototypeApp.tsx");
const source = fs.readFileSync(appPath, "utf8");

const checks = [
  ["data tabs marker", /data-v245-data-tabs="true"/.test(source)],
  ["data bottom marker", /data-v245-data-bottom="true"/.test(source)],
  ["variable view rows", /tab === "Variable View"/.test(source) && /Data Quality/.test(source) && /Import History/.test(source)],
  ["model live tree marker", /data-v245-model-live-tree="true"/.test(source)],
  ["model project binding", /data\.projectSummary\.name/.test(source) && /data\.projectSummary\.dataset/.test(source)],
  ["model bottom tabs marker", /data-v245-model-bottom-tabs="true"/.test(source)],
  ["model bottom tab switching", /setBottomTab/.test(source) && /Diagram Advisor/.test(source) && /Calculation Log/.test(source) && /Output/.test(source)],
  ["setup drawer marker", /data-v245-setup-drawer="true"/.test(source)],
  ["setup project counts", /data\.projectSummary\.constructs/.test(source) && /data\.projectSummary\.indicators/.test(source) && /data\.projectSummary\.paths/.test(source)],
  ["run output preview marker", /data-v245-run-output-preview="true"/.test(source)],
  ["trust integrity marker", /data-v245-trust-integrity="true"/.test(source)],
  ["trust checksum command", /verifyReleaseChecksumsFromShell/.test(source)],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  milestone: "v2_45_0_mockup_visible_feature_completion",
  passed: failures.length === 0,
  checks: Object.fromEntries(checks),
  failures,
};

const outputPath = path.join(root, "validation", "results", "v245_mockup_feature_completion_smoke.json");
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));

if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}

console.log(JSON.stringify(result, null, 2));
