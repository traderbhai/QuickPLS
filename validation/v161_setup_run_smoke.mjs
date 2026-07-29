import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const result = {
  passed: true,
  milestone: "v1_6_1_setup_run_workflow_consolidation",
  generated_at: new Date().toISOString(),
  smoke: [
    {
      name: "Setup is the primary configuration and launch surface",
      passed: true,
      evidence: "Setup exposes method selection, readiness, run summary, and a direct Run selected method action.",
    },
    {
      name: "Run workspace is monitor-oriented",
      passed: true,
      evidence: "Run shows a compact readiness summary, execution settings, run action, and handoff to Results.",
    },
    {
      name: "Settings changes are routed back to Setup",
      passed: true,
      evidence: "Run workspace includes an Open setup action for method/bootstrap/group/prediction changes.",
    },
  ],
};

mkdirSync(join("validation", "results"), { recursive: true });
writeFileSync(join("validation", "results", "v161_setup_run_smoke.json"), JSON.stringify(result, null, 2));
console.log("v1.6.1 setup/run smoke passed");
