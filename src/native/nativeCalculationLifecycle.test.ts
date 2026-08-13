import { describe, expect, it } from "vitest";
import type { RunMonitorState, RunMonitorStatus } from "../types";
import {
  InvalidCalculationTransitionError,
  NATIVE_CALCULATION_STATUSES,
  isAllowedCalculationTransition,
  isCalculationActive,
  nativeCalculationPhaseLabel,
  transitionCalculationMonitor,
} from "./nativeCalculationLifecycle";

function monitor(status: RunMonitorStatus, completedUnits = 0): RunMonitorState {
  return {
    status,
    phase: status,
    message: status,
    completedUnits,
    totalUnits: 5,
    startedAt: null,
    completedAt: null,
    activeJobId: null,
    lastRunId: null,
    error: null,
    logs: [],
  };
}

function follow(initial: RunMonitorStatus, ...statuses: RunMonitorStatus[]): RunMonitorState {
  return statuses.reduce(
    (current, status) => transitionCalculationMonitor(current, { status }),
    monitor(initial),
  );
}

describe("native calculation lifecycle", () => {
  it("formats native phases for professional user-facing status labels", () => {
    expect(nativeCalculationPhaseLabel("iterating", "running")).toBe("Iterating");
    expect(nativeCalculationPhaseLabel("group_permutation", "running")).toBe("Group permutation");
    expect(nativeCalculationPhaseLabel("", "cancelling")).toBe("Cancelling");
  });

  it("keeps the transition contract exhaustive for every monitor status", () => {
    expect(NATIVE_CALCULATION_STATUSES).toEqual([
      "idle",
      "blocked",
      "queued",
      "validating",
      "running",
      "cancelling",
      "completed",
      "failed",
      "cancelled",
    ]);

    for (const status of NATIVE_CALCULATION_STATUSES) {
      expect(isAllowedCalculationTransition(status, status)).toBe(true);
    }
  });

  it("allows the successful queued, validation, engine, and completion path", () => {
    expect(follow("idle", "queued", "validating", "queued", "running", "completed").status).toBe("completed");
  });

  it("allows a blocked attempt to be retried after inputs become ready", () => {
    expect(follow("idle", "blocked", "queued", "validating", "running", "completed").status).toBe("completed");
  });

  it("allows failures from each active phase and a clean retry", () => {
    for (const status of ["queued", "validating", "running", "cancelling"] as const) {
      const failed = transitionCalculationMonitor(monitor(status), {
        status: "failed",
        error: "Engine failed",
      });
      expect(failed.error).toBe("Engine failed");
      expect(follow(failed.status, "queued", "validating", "running", "completed").status).toBe("completed");
    }
  });

  it("covers cancellation, a completed-before-cancel race, and retry", () => {
    expect(follow("running", "cancelling", "cancelled", "queued").status).toBe("queued");
    expect(follow("running", "cancelling", "completed").status).toBe("completed");
    expect(follow("running", "cancelling", "running").status).toBe("running");
  });

  it("allows same-state progress without losing prior monitor data", () => {
    const current = { ...monitor("running", 1), activeJobId: "job-1", startedAt: "2026-08-10T00:00:00.000Z" };
    const next = transitionCalculationMonitor(current, {
      status: "running",
      completedUnits: 2,
      message: "Iteration 2",
    });

    expect(next).toMatchObject({
      status: "running",
      completedUnits: 2,
      totalUnits: 5,
      activeJobId: "job-1",
      startedAt: "2026-08-10T00:00:00.000Z",
      message: "Iteration 2",
    });
  });

  it("identifies only non-terminal calculation states as active", () => {
    expect(NATIVE_CALCULATION_STATUSES.filter(isCalculationActive)).toEqual([
      "queued",
      "validating",
      "running",
      "cancelling",
    ]);
  });

  it.each([
    ["idle", "completed"],
    ["blocked", "running"],
    ["completed", "running"],
    ["failed", "completed"],
    ["cancelled", "completed"],
  ] as const)("rejects the invalid %s -> %s transition", (from, to) => {
    expect(() => transitionCalculationMonitor(monitor(from), { status: to }))
      .toThrow(InvalidCalculationTransitionError);
  });
});
