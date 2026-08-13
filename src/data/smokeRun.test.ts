import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "./smokeRun";

describe("completed sample PLS bootstrap run", () => {
  it("truthfully labels its embedded bootstrap output without changing its versioned fixture identity", () => {
    const run = completedSamplePlsRun();

    expect(run).toMatchObject({
      id: "v11-smoke-completed-pls",
      name: "PLS-SEM Bootstrapping run",
      method: "PLS-SEM Bootstrapping",
    });
    expect(run.bootstrap?.percentile.parameters.length).toBeGreaterThan(0);
    expect(`${run.name} ${run.method}`).not.toMatch(/v0\.4|demo evidence/i);
  });
});
