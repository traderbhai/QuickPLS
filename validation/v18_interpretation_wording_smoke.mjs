import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultsSrc = fs.readFileSync(path.join(root, "src", "components", "RunHistory.tsx"), "utf8");
const interpretationSrc = fs.readFileSync(path.join(root, "src", "domain", "resultInterpretation.ts"), "utf8");
const checks = [
  ["canonical dedupe", interpretationSrc.includes("canonicalFindingKey") && interpretationSrc.includes("htmtPairKey")],
  ["finding structure says what value says", resultsSrc.includes("What the value says")],
  ["finding structure says why it matters", resultsSrc.includes("Why it matters")],
  ["finding structure says what to inspect", resultsSrc.includes("What to inspect next")],
  ["finding structure report wording", resultsSrc.includes("Report wording")],
  ["conservative language", interpretationSrc.includes("not establish causality") || interpretationSrc.includes("avoid p-value")],
  ["no mojibake", !resultsSrc.includes("RÂ²") && !interpretationSrc.includes("RÂ²")],
];
const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const out = {
  passed: failed.length === 0,
  milestone: "v1_8_results_report_refinement_real_user_testing",
  checks: Object.fromEntries(checks),
  failed,
};
const outPath = path.join(root, "validation", "results", "v18_interpretation_wording_smoke.json");
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
if (failed.length) {
  console.error(`v1.8 interpretation wording smoke failed: ${failed.join(", ")}`);
  process.exit(1);
}
console.log(`wrote ${outPath}`);
