import fs from "node:fs";

const checks = [];
const add = (name, passed, detail) => checks.push({ name, passed, detail });
const read = (path) => fs.readFileSync(path, "utf8");

const trust = read("src/components/TrustCenterWorkspace.tsx");
const styles = read("src/styles.css");
const applicability = read("src/domain/methodApplicability.ts");
const pkg = JSON.parse(read("package.json"));
const topBar = read("src/components/TopBar.tsx");

const requiredTrustTokens = [
  "trust-v2-workspace",
  "trust-v2-hero",
  "Why trust this result?",
  "Validation artifact index",
  "Method scope and applicability",
  "Offline and legal boundary",
  "No SmartPLS equivalence",
  "methodApplicabilityFor",
  "Dataset fingerprint",
  "Latest run",
];

for (const token of requiredTrustTokens) {
  add(`Trust Center contains ${token}`, trust.includes(token), token);
}

const requiredStyleTokens = [
  ".trust-v2-workspace",
  ".trust-v2-hero",
  ".trust-v2-current-method",
  ".trust-v2-confidence-grid",
  ".trust-v2-policy-grid",
];

for (const token of requiredStyleTokens) {
  add(`Styles contain ${token}`, styles.includes(token), token);
}

add("package version is 2.0.8", pkg.version === "2.0.8", pkg.version);
add("release artifact label is v2.0.8", pkg.scripts["qpls:release:artifacts"].includes("v2_0_8_trust_center_scope_transparency"), pkg.scripts["qpls:release:artifacts"]);
add("top bar label is current", topBar.includes("v2.0.8 trust center redesign"), "TopBar alpha mark");
add("Trust Center source has no mojibake", !/[ÃƒÃ‚ï¿½Ã¯Â¿Â½]|RÂ²/.test(trust), "TrustCenter encoding");
add("Applicability output map uses R²", applicability.includes("R²") && !applicability.includes("RÂ²"), "methodApplicability R² text");

const passed = checks.every((check) => check.passed);
const result = {
  passed,
  milestone: "v2_0_8_trust_center_scope_transparency",
  generated_at: new Date().toISOString(),
  checks,
};

fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync("validation/results/v208_trust_center_smoke.json", JSON.stringify(result, null, 2));

if (!passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}

console.log("v2.0.8 Trust Center smoke passed");
