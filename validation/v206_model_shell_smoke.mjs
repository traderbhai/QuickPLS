import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const outPath = path.join(root, "validation", "results", "v206_model_shell_smoke.json");

const sources = {
  packageJson: read("package.json"),
  explorer: read("src/components/Explorer.tsx"),
  canvas: read("src/components/ModelCanvas.tsx"),
  inspector: read("src/components/Inspector.tsx"),
  styles: read("src/styles.css"),
  topbar: read("src/components/TopBar.tsx"),
  contract: read("docs/V2_UI_VISUAL_CONTRACT.md"),
};

const checks = [
  ["package version is 2.0.6", sources.packageJson.includes('"version": "2.0.6"')],
  ["release artifact label is v2.0.6", sources.packageJson.includes("v2_0_6_model_shell_sem_designer_surround")],
  ["top bar shows v2.0.6 model shell label", sources.topbar.includes("v2.0.6 model shell redesign")],
  ["Explorer exposes v2 shell hooks", ["model-v2-explorer", "model-v2-status-card", "model-v2-guidance-card", "model-v2-tabs"].every((token) => sources.explorer.includes(token))],
  ["Model canvas exposes v2 shell hooks", ["model-v2-canvas", "model-v2-toolbar"].every((token) => sources.canvas.includes(token))],
  ["Inspector exposes v2 shell hook", sources.inspector.includes("model-v2-inspector")],
  ["Styles include v2 model shell selectors", [".model-v2-explorer", ".model-v2-canvas", ".model-v2-toolbar", ".model-v2-inspector", ".workspace-shell:has(.model-v2-canvas)"].every((token) => sources.styles.includes(token))],
  ["Model shell keeps R² readable", sources.canvas.includes("R²") && sources.inspector.includes("R²")],
  ["No v2 Model shell mojibake markers", ![sources.canvas, sources.inspector, sources.styles, sources.contract].join("\n").match(/RÂ|RÃ|R�|Ã‚|ï¿½/)],
  ["Visual contract states frontend-only boundary", sources.contract.includes("product/UI-only") && sources.contract.includes("numerical fingerprints")],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_0_6_model_shell_sem_designer_surround",
  checked_at: new Date().toISOString(),
  checks: checks.map(([name, passed]) => ({ name, passed })),
  failures,
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(result, null, 2)}\n`);

if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}

console.log(JSON.stringify(result, null, 2));
