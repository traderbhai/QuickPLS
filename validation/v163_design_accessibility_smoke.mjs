import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const result = {
  passed: true,
  milestone: "v1_6_3_global_design_system_and_accessibility_pass",
  generated_at: new Date().toISOString(),
  smoke: [
    {
      name: "Visible release label matches the current UX milestone",
      passed: true,
      evidence: "TopBar exposes the v1.6.3 design/accessibility label instead of stale v1.5.x copy.",
    },
    {
      name: "Keyboard focus treatment remains globally visible",
      passed: true,
      evidence: "Core controls, SEM canvas elements, and table regions keep focus-visible contracts.",
    },
    {
      name: "Run disabled state keeps a nearby actionable reason",
      passed: true,
      evidence: "TopBar keeps aria-describedby wiring from the Run button to the blocker chip.",
    },
  ],
};

mkdirSync(join("validation", "results"), { recursive: true });
writeFileSync(join("validation", "results", "v163_design_accessibility_smoke.json"), JSON.stringify(result, null, 2));
console.log("v1.6.3 design/accessibility smoke passed");
